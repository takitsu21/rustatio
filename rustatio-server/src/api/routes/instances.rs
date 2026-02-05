//! Instance management endpoints.

use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::Response,
    routing::{delete, get, patch, post},
    Json, Router,
};
use rustatio_core::{FakerConfig, TorrentInfo};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::api::{
    common::{ApiError, ApiSuccess, EmptyData},
    routes::torrents::LoadTorrentResponse,
    ServerState,
};
use crate::services::persistence::InstanceSource;
use crate::services::InstanceInfo;

#[derive(serde::Serialize, ToSchema)]
pub struct CreateInstanceResponse {
    pub id: String,
}

#[derive(Deserialize, ToSchema)]
pub struct DeleteInstanceQuery {
    #[serde(default)]
    pub force: bool,
}

#[utoipa::path(
    post,
    path = "/instances",
    tag = "instances",
    summary = "Create a new instance",
    description = "Generates a new unique instance ID. The instance is not started until you call the start endpoint.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Instance ID created", body = ApiSuccess<CreateInstanceResponse>),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
pub async fn create_instance(State(state): State<ServerState>) -> Response {
    let id = state.app.next_instance_id().await;
    ApiSuccess::response(CreateInstanceResponse { id })
}

#[utoipa::path(
    get,
    path = "/instances",
    tag = "instances",
    summary = "List all instances",
    description = "Returns a list of all faker instances with their current statistics and configuration.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of instances", body = ApiSuccess<Vec<InstanceInfo>>),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
pub async fn list_instances(State(state): State<ServerState>) -> Response {
    let instances: Vec<InstanceInfo> = state.app.list_instances().await;
    ApiSuccess::response(instances)
}

#[utoipa::path(
    delete,
    path = "/instances/{id}",
    tag = "instances",
    summary = "Delete an instance",
    description = "Deletes a faker instance. Watch folder instances require force=true to delete.",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Instance ID to delete"),
        ("force" = Option<bool>, Query, description = "Force delete even for watch folder instances")
    ),
    responses(
        (status = 200, description = "Instance deleted", body = ApiSuccess<EmptyData>),
        (status = 400, description = "Cannot delete watch folder instance without force", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
pub async fn delete_instance(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Query(query): Query<DeleteInstanceQuery>,
) -> Response {
    let instance_info = state.app.get_instance_info_for_delete(&id).await;

    match state.app.delete_instance(&id, query.force).await {
        Ok(()) => {
            if query.force {
                if let Some((source, info_hash)) = instance_info {
                    if source == InstanceSource::WatchFolder {
                        state.watch.write().await.remove_info_hash(&info_hash).await;
                    }
                }
            }
            ApiSuccess::response(EmptyData {})
        }
        Err(e) => ApiError::response(StatusCode::BAD_REQUEST, e),
    }
}

#[utoipa::path(
    post,
    path = "/instances/{id}/torrent",
    tag = "instances",
    summary = "Load torrent for instance",
    description = "Uploads a torrent file and associates it with the specified instance ID. Creates an idle instance that persists across page refreshes.",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Instance ID to associate with the torrent")
    ),
    request_body(content_type = "multipart/form-data", description = "Torrent file upload (field name: file)"),
    responses(
        (status = 200, description = "Torrent loaded and instance created", body = ApiSuccess<LoadTorrentResponse>),
        (status = 400, description = "Invalid torrent file", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Failed to create instance", body = ApiError)
    )
)]
pub async fn load_instance_torrent(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Response {
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            match field.bytes().await {
                Ok(bytes) => match TorrentInfo::from_bytes(&bytes) {
                    Ok(torrent) => {
                        if state.app.instance_exists(&id).await {
                            return ApiSuccess::response(LoadTorrentResponse {
                                torrent_id: id,
                                torrent,
                            });
                        }

                        if let Err(e) = state.app.create_idle_instance(&id, torrent.clone()).await {
                            return ApiError::response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                format!("Failed to create instance: {}", e),
                            );
                        }

                        return ApiSuccess::response(LoadTorrentResponse {
                            torrent_id: id,
                            torrent,
                        });
                    }
                    Err(e) => {
                        return ApiError::response(StatusCode::BAD_REQUEST, format!("Failed to parse torrent: {}", e));
                    }
                },
                Err(e) => {
                    return ApiError::response(StatusCode::BAD_REQUEST, format!("Failed to read file: {}", e));
                }
            }
        }
    }

    ApiError::response(StatusCode::BAD_REQUEST, "No torrent file provided")
}

#[utoipa::path(
    patch,
    path = "/instances/{id}/config",
    tag = "instances",
    summary = "Update instance configuration",
    description = "Updates the configuration for an existing instance without starting it. Used to persist form changes.",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Instance ID to update")
    ),
    request_body(content = Object, description = "Faker configuration settings"),
    responses(
        (status = 200, description = "Configuration updated", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Instance not found", body = ApiError)
    )
)]
pub async fn update_instance_config(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(config): Json<FakerConfig>,
) -> Response {
    match state.app.update_instance_config_only(&id, config).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/instances", get(list_instances).post(create_instance))
        .route("/instances/{id}", delete(delete_instance))
        .route("/instances/{id}/torrent", post(load_instance_torrent))
        .route("/instances/{id}/config", patch(update_instance_config))
}
