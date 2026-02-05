//! Watch folder management endpoints.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    routing::{delete, get, post},
    Router,
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::api::{
    common::{ApiError, ApiSuccess, EmptyData},
    ServerState,
};
use crate::services::watch::{WatchStatus, WatchedFile};

#[derive(Serialize, ToSchema)]
pub struct ReloadAllResponse {
    pub reloaded: u32,
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
    path = "/watch/files/{filename}",
    tag = "watch",
    summary = "Delete watch folder file",
    description = "Deletes a torrent file from the watch folder and removes its corresponding instance.",
    security(("bearer_auth" = [])),
    params(
        ("filename" = String, Path, description = "Filename to delete")
    ),
    responses(
        (status = 200, description = "File deleted", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "File not found", body = ApiError)
    )
)]
pub async fn delete_watch_file(State(state): State<ServerState>, Path(filename): Path<String>) -> Response {
    let watch = state.watch.read().await;
    match watch.delete_file(&filename).await {
        Ok(()) => ApiSuccess::response(EmptyData {}),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

#[utoipa::path(
    post,
    path = "/watch/files/{filename}/reload",
    tag = "watch",
    summary = "Reload watch folder file",
    description = "Reloads a single torrent file from the watch folder, removing and recreating its instance.",
    security(("bearer_auth" = [])),
    params(
        ("filename" = String, Path, description = "Filename to reload")
    ),
    responses(
        (status = 200, description = "File reloaded", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Failed to reload file", body = ApiError)
    )
)]
pub async fn reload_watch_file(State(state): State<ServerState>, Path(filename): Path<String>) -> Response {
    let watch = state.watch.read().await;
    match watch.reload_file(&filename).await {
        Ok(()) => ApiSuccess::response(EmptyData {}),
        Err(e) => ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
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
        .route("/watch/files/{filename}", delete(delete_watch_file))
        .route("/watch/files/{filename}/reload", post(reload_watch_file))
        .route("/watch/reload", post(reload_all_watch_files))
}
