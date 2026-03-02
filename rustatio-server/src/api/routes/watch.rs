//! Watch folder management endpoints.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Response,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::{
    common::{ApiError, ApiSuccess, EmptyData},
    ServerState,
};
use crate::services::persistence::WatchSettings;
use crate::services::watch::{WatchConfig, WatchStatus, WatchedFile};

#[derive(Serialize, ToSchema)]
pub struct ReloadAllResponse {
    pub reloaded: u32,
}

#[derive(Serialize, ToSchema)]
pub struct WatchConfigResponse {
    pub max_depth: u32,
    pub auto_start: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WatchConfigRequest {
    pub max_depth: u32,
    pub auto_start: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WatchPathQuery {
    pub path: String,
}

#[utoipa::path(
    get,
    path = "/watch/status",
    tag = "watch",
    summary = "Get watch folder status",
    description = "Returns the current status of the watch folder service including whether it's enabled and auto-start settings.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Watch folder status", body = ApiSuccess<WatchStatus>),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
pub async fn get_watch_status(State(state): State<ServerState>) -> Response {
    let watch = state.watch.read().await;
    let status: WatchStatus = watch.get_status().await;
    ApiSuccess::response(status)
}

#[utoipa::path(
    get,
    path = "/watch/files",
    tag = "watch",
    summary = "List watch folder files",
    description = "Returns a list of all torrent files in the watch folder with their current status.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of watched files", body = ApiSuccess<Vec<WatchedFile>>),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
pub async fn list_watch_files(State(state): State<ServerState>) -> Response {
    let watch = state.watch.read().await;
    let files: Vec<WatchedFile> = watch.list_files().await;
    ApiSuccess::response(files)
}

#[utoipa::path(
    delete,
    path = "/watch/files",
    tag = "watch",
    summary = "Delete watch folder file",
    description = "Deletes a torrent file from the watch folder and removes its corresponding instance.",
    security(("bearer_auth" = [])),
    params(
        ("path" = String, Query, description = "Relative path to delete")
    ),
    responses(
        (status = 200, description = "File deleted", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "File not found", body = ApiError)
    )
)]
pub async fn delete_watch_file_by_path(
    State(state): State<ServerState>,
    Query(query): Query<WatchPathQuery>,
) -> Response {
    let watch = state.watch.read().await;
    match watch.delete_file(&query.path).await {
        Ok(()) => ApiSuccess::response(EmptyData {}),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

#[utoipa::path(
    post,
    path = "/watch/files/reload",
    tag = "watch",
    summary = "Reload watch folder file",
    description = "Reloads a single torrent file from the watch folder, removing and recreating its instance.",
    security(("bearer_auth" = [])),
    params(
        ("path" = String, Query, description = "Relative path to reload")
    ),
    responses(
        (status = 200, description = "File reloaded", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Failed to reload file", body = ApiError)
    )
)]
pub async fn reload_watch_file_by_path(
    State(state): State<ServerState>,
    Query(query): Query<WatchPathQuery>,
) -> Response {
    let watch = state.watch.read().await;
    match watch.reload_file(&query.path).await {
        Ok(()) => ApiSuccess::response(EmptyData {}),
        Err(e) => ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

#[utoipa::path(
    get,
    path = "/watch/config",
    tag = "watch",
    summary = "Get watch folder configuration",
    description = "Returns the current watch folder configuration settings.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Watch folder configuration", body = ApiSuccess<WatchConfigResponse>),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
pub async fn get_watch_config(State(state): State<ServerState>) -> Response {
    let watch = state.watch.read().await;
    let config: WatchConfig = watch.config();
    ApiSuccess::response(WatchConfigResponse {
        max_depth: config.max_depth,
        auto_start: config.auto_start,
    })
}

#[utoipa::path(
    put,
    path = "/watch/config",
    tag = "watch",
    summary = "Update watch folder configuration",
    description = "Updates watch folder configuration settings.",
    security(("bearer_auth" = [])),
    request_body(content = WatchConfigRequest, description = "Watch folder settings"),
    responses(
        (status = 200, description = "Watch folder configuration updated", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Failed to update watch configuration", body = ApiError)
    )
)]
pub async fn set_watch_config(
    State(state): State<ServerState>,
    axum::Json(payload): axum::Json<WatchConfigRequest>,
) -> Response {
    let max_depth = payload.max_depth;
    let auto_start = payload.auto_start;
    let settings = WatchSettings { max_depth, auto_start };
    if let Err(e) = state.app.set_watch_settings(settings).await {
        return ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e);
    }

    let mut watch = state.watch.write().await;
    watch.set_max_depth(max_depth);
    watch.set_auto_start(auto_start);
    ApiSuccess::response(EmptyData {})
}

#[utoipa::path(
    post,
    path = "/watch/reload",
    tag = "watch",
    summary = "Reload all watch folder files",
    description = "Scans the watch folder and loads any new or previously unloaded torrent files.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Files reloaded", body = ApiSuccess<ReloadAllResponse>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Failed to reload files", body = ApiError)
    )
)]
pub async fn reload_all_watch_files(State(state): State<ServerState>) -> Response {
    let watch = state.watch.read().await;
    match watch.reload_all().await {
        Ok(count) => ApiSuccess::response(ReloadAllResponse { reloaded: count }),
        Err(e) => ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/watch/status", get(get_watch_status))
        .route("/watch/files", get(list_watch_files))
        .route("/watch/files", delete(delete_watch_file_by_path))
        .route("/watch/files/reload", post(reload_watch_file_by_path))
        .route("/watch/reload", post(reload_all_watch_files))
        .route("/watch/config", get(get_watch_config))
        .route("/watch/config", put(set_watch_config))
}
