use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::StatusCode,
    response::Response,
    routing::{get, post, put},
    Json, Router,
};
use rustatio_core::{FakerConfig, GridImportSettings, InstanceSummary, PresetSettings, TorrentInfo};
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::api::{
    common::{ApiError, ApiSuccess},
    ServerState,
};
use crate::services::events::{EventBroadcaster, InstanceEvent};
use crate::services::persistence::InstanceSource;
use crate::services::InstanceLifecycle;

#[derive(Deserialize)]
pub struct GridIdsRequest {
    pub ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct GridTagRequest {
    pub ids: Vec<String>,
    #[serde(default)]
    pub add_tags: Vec<String>,
    #[serde(default)]
    pub remove_tags: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridUpdateConfigRequest {
    pub ids: Vec<String>,
    pub config: PresetSettings,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridImportFolderRequest {
    pub path: String,
    #[serde(default)]
    pub config: GridImportSettings,
}

#[derive(Deserialize)]
pub struct GridBulkUpdateConfigEntry {
    pub id: String,
    pub config: FakerConfig,
}

#[derive(Serialize)]
pub struct GridImportResponse {
    pub imported: Vec<GridImportedInstance>,
    pub errors: Vec<String>,
}

#[derive(Serialize)]
pub struct GridImportedInstance {
    pub id: String,
    pub name: String,
    pub info_hash: String,
}

#[derive(Serialize)]
pub struct GridActionResponse {
    pub succeeded: Vec<String>,
    pub failed: Vec<GridActionError>,
}

#[derive(Serialize)]
pub struct GridActionError {
    pub id: String,
    pub error: String,
}

pub async fn grid_import(State(state): State<ServerState>, mut multipart: Multipart) -> Response {
    let mut torrents: Vec<(String, TorrentInfo)> = Vec::new();
    let mut config = GridImportSettings::default();
    let mut errors: Vec<String> = Vec::new();

    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => match field.name() {
                Some("config") => match field.text().await {
                    Ok(text) => match serde_json::from_str::<GridImportSettings>(&text) {
                        Ok(c) => config = c,
                        Err(e) => {
                            return ApiError::response(StatusCode::BAD_REQUEST, format!("Invalid config JSON: {}", e));
                        }
                    },
                    Err(e) => {
                        return ApiError::response(
                            StatusCode::BAD_REQUEST,
                            format!("Failed to read config field: {}", e),
                        );
                    }
                },
                Some("files") | Some("file") => {
                    let filename = field.file_name().unwrap_or("unknown").to_string();
                    match field.bytes().await {
                        Ok(bytes) => match TorrentInfo::from_bytes(&bytes) {
                            Ok(torrent) => {
                                let id = state.app.next_instance_id().await;
                                torrents.push((id, torrent));
                            }
                            Err(e) => {
                                errors.push(format!("{}: {}", filename, e));
                            }
                        },
                        Err(e) => {
                            errors.push(format!("{}: failed to read: {}", filename, e));
                        }
                    }
                }
                _ => {}
            },
            Ok(None) => break,
            Err(e) => {
                errors.push(format!("Multipart parse error: {}", e));
                break;
            }
        }
    }

    if torrents.is_empty() && errors.is_empty() {
        return ApiError::response(StatusCode::BAD_REQUEST, "No torrent files provided");
    }

    let mut imported = Vec::new();

    for (id, torrent) in &torrents {
        let preset = config.resolve_for_instance();
        let faker_config: FakerConfig = preset.into();

        match state
            .app
            .create_instance_with_tags(
                id,
                torrent.clone(),
                faker_config,
                config.tags.clone(),
                InstanceSource::Manual,
            )
            .await
        {
            Ok(()) => {
                imported.push(GridImportedInstance {
                    id: id.clone(),
                    name: torrent.name.clone(),
                    info_hash: hex::encode(torrent.info_hash),
                });

                state.app.emit_instance_event(InstanceEvent::Created {
                    id: id.clone(),
                    torrent_name: torrent.name.clone(),
                    info_hash: hex::encode(torrent.info_hash),
                    auto_started: config.auto_start,
                });
            }
            Err(e) => {
                errors.push(format!("{}: {}", torrent.name, e));
            }
        }
    }

    if config.auto_start {
        for inst in &imported {
            if let Some(stagger) = config.stagger_start_secs {
                if stagger > 0 {
                    let idx = imported.iter().position(|i| i.id == inst.id).unwrap_or(0);
                    if idx > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_secs(stagger)).await;
                    }
                }
            }
            if let Err(e) = state.app.start_instance(&inst.id).await {
                errors.push(format!("{}: failed to start: {}", inst.name, e));
            }
        }
    }

    ApiSuccess::response(GridImportResponse { imported, errors })
}

pub async fn grid_import_folder(
    State(state): State<ServerState>,
    Json(request): Json<GridImportFolderRequest>,
) -> Response {
    let path = std::path::Path::new(&request.path);
    if !path.exists() || !path.is_dir() {
        return ApiError::response(
            StatusCode::BAD_REQUEST,
            format!("Directory not found: {}", request.path),
        );
    }

    let mut torrents: Vec<(String, TorrentInfo)> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(e) => {
            return ApiError::response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read directory: {}", e),
            );
        }
    };

    for entry in entries.flatten() {
        let file_path = entry.path();
        if file_path.extension().and_then(|e| e.to_str()) != Some("torrent") {
            continue;
        }

        let filename = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();

        match std::fs::read(&file_path) {
            Ok(bytes) => match TorrentInfo::from_bytes(&bytes) {
                Ok(torrent) => {
                    // Skip if already imported (same info_hash)
                    if state.app.find_instance_by_info_hash(&torrent.info_hash).await.is_some() {
                        errors.push(format!("{}: already imported", filename));
                        continue;
                    }
                    let id = state.app.next_instance_id().await;
                    torrents.push((id, torrent));
                }
                Err(e) => {
                    errors.push(format!("{}: {}", filename, e));
                }
            },
            Err(e) => {
                errors.push(format!("{}: failed to read: {}", filename, e));
            }
        }
    }

    let config = request.config;
    let mut imported = Vec::new();

    for (id, torrent) in &torrents {
        let preset = config.resolve_for_instance();
        let faker_config: FakerConfig = preset.into();

        match state
            .app
            .create_instance_with_tags(
                id,
                torrent.clone(),
                faker_config,
                config.tags.clone(),
                InstanceSource::Manual,
            )
            .await
        {
            Ok(()) => {
                imported.push(GridImportedInstance {
                    id: id.clone(),
                    name: torrent.name.clone(),
                    info_hash: hex::encode(torrent.info_hash),
                });

                state.app.emit_instance_event(InstanceEvent::Created {
                    id: id.clone(),
                    torrent_name: torrent.name.clone(),
                    info_hash: hex::encode(torrent.info_hash),
                    auto_started: config.auto_start,
                });
            }
            Err(e) => {
                errors.push(format!("{}: {}", torrent.name, e));
            }
        }
    }

    if config.auto_start {
        for (idx, inst) in imported.iter().enumerate() {
            if let Some(stagger) = config.stagger_start_secs {
                if stagger > 0 && idx > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_secs(stagger)).await;
                }
            }
            if let Err(e) = state.app.start_instance(&inst.id).await {
                errors.push(format!("{}: failed to start: {}", inst.name, e));
            }
        }
    }

    ApiSuccess::response(GridImportResponse { imported, errors })
}

pub async fn grid_start(State(state): State<ServerState>, Json(request): Json<GridIdsRequest>) -> Response {
    let ids = request.ids;
    tokio::spawn(async move {
        for id in &ids {
            if let Err(e) = state.app.start_instance(id).await {
                tracing::warn!("Background grid_start failed for {}: {}", id, e);
            }
        }
    });
    ApiSuccess::response(GridActionResponse {
        succeeded: Vec::new(),
        failed: Vec::new(),
    })
}

pub async fn grid_stop(State(state): State<ServerState>, Json(request): Json<GridIdsRequest>) -> Response {
    let ids = request.ids;
    tokio::spawn(async move {
        for id in &ids {
            if let Err(e) = state.app.stop_instance(id).await {
                tracing::warn!("Background grid_stop failed for {}: {}", id, e);
            }
        }
    });
    ApiSuccess::response(GridActionResponse {
        succeeded: Vec::new(),
        failed: Vec::new(),
    })
}

pub async fn grid_pause(State(state): State<ServerState>, Json(request): Json<GridIdsRequest>) -> Response {
    let (succeeded, failed) = grid_action_concurrent(&state, request.ids, GridAction::Pause).await;
    ApiSuccess::response(GridActionResponse { succeeded, failed })
}

pub async fn grid_resume(State(state): State<ServerState>, Json(request): Json<GridIdsRequest>) -> Response {
    let (succeeded, failed) = grid_action_concurrent(&state, request.ids, GridAction::Resume).await;
    ApiSuccess::response(GridActionResponse { succeeded, failed })
}

pub async fn grid_delete(State(state): State<ServerState>, Json(request): Json<GridIdsRequest>) -> Response {
    let (succeeded, failed) = grid_action_concurrent(&state, request.ids, GridAction::Delete).await;
    ApiSuccess::response(GridActionResponse { succeeded, failed })
}

pub async fn grid_update_config(
    State(state): State<ServerState>,
    Json(request): Json<GridUpdateConfigRequest>,
) -> Response {
    let faker_config: FakerConfig = request.config.into();

    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    for id in &request.ids {
        match state.app.update_instance_config(id, faker_config.clone()).await {
            Ok(()) => succeeded.push(id.clone()),
            Err(e) => failed.push(GridActionError {
                id: id.clone(),
                error: e,
            }),
        }
    }

    ApiSuccess::response(GridActionResponse { succeeded, failed })
}

pub async fn grid_tag(State(state): State<ServerState>, Json(request): Json<GridTagRequest>) -> Response {
    match state
        .app
        .grid_update_tags(&request.ids, &request.add_tags, &request.remove_tags)
        .await
    {
        Ok(count) => ApiSuccess::response(serde_json::json!({ "updated": count })),
        Err(e) => ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

pub async fn list_summaries(State(state): State<ServerState>) -> Response {
    let summaries: Vec<InstanceSummary> = state.app.list_instance_summaries().await;
    ApiSuccess::response(summaries)
}

#[derive(Deserialize)]
pub struct SetTagsRequest {
    pub tags: Vec<String>,
}

pub async fn set_instance_tags(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(request): Json<SetTagsRequest>,
) -> Response {
    match state.app.update_instance_tags(&id, request.tags).await {
        Ok(()) => ApiSuccess::response(serde_json::json!({ "id": id })),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

pub async fn grid_bulk_update_configs(
    State(state): State<ServerState>,
    Json(entries): Json<Vec<GridBulkUpdateConfigEntry>>,
) -> Response {
    let pairs: Vec<(String, FakerConfig)> = entries.into_iter().map(|e| (e.id, e.config)).collect();
    let (succeeded, failed_pairs) = state.app.bulk_update_configs(pairs).await;

    let failed = failed_pairs
        .into_iter()
        .map(|(id, error)| GridActionError { id, error })
        .collect();

    ApiSuccess::response(GridActionResponse { succeeded, failed })
}

enum GridAction {
    Pause,
    Resume,
    Delete,
}

async fn grid_action_concurrent(
    state: &ServerState,
    ids: Vec<String>,
    action: GridAction,
) -> (Vec<String>, Vec<GridActionError>) {
    let mut set = JoinSet::new();

    for id in ids {
        let state = state.clone();
        let action_kind = match action {
            GridAction::Pause => 0u8,
            GridAction::Resume => 1,
            GridAction::Delete => 2,
        };
        set.spawn(async move {
            let result = match action_kind {
                0 => state.app.pause_instance(&id).await,
                1 => state.app.resume_instance(&id).await,
                _ => state.app.delete_instance(&id, true).await,
            };
            (id, result)
        });
    }

    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    while let Some(join_result) = set.join_next().await {
        match join_result {
            Ok((id, Ok(()))) => succeeded.push(id),
            Ok((id, Err(e))) => failed.push(GridActionError { id, error: e }),
            Err(e) => {
                tracing::warn!("Grid action task panicked: {}", e);
            }
        }
    }

    (succeeded, failed)
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/grid/import", post(grid_import))
        .layer(DefaultBodyLimit::max(512 * 1024 * 1024))
        .route("/grid/import-folder", post(grid_import_folder))
        .route("/grid/start", post(grid_start))
        .route("/grid/stop", post(grid_stop))
        .route("/grid/pause", post(grid_pause))
        .route("/grid/resume", post(grid_resume))
        .route("/grid/delete", post(grid_delete))
        .route("/grid/update-config", post(grid_update_config))
        .route("/grid/bulk-update-configs", post(grid_bulk_update_configs))
        .route("/grid/tag", post(grid_tag))
        .route("/instances/summary", get(list_summaries))
        .route("/instances/{id}/tags", put(set_instance_tags))
}
