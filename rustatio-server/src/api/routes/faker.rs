//! Faker control endpoints (start, stop, pause, resume, update, stats).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use rustatio_core::{FakerConfig, TorrentInfo};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::api::{
    common::{ApiError, ApiSuccess, EmptyData},
    ServerState,
};
use crate::services::InstanceLifecycle;

#[derive(Deserialize, ToSchema)]
pub struct StartFakerRequest {
    #[schema(value_type = Object)]
    pub torrent: TorrentInfo,
    #[schema(value_type = Object)]
    pub config: FakerConfig,
}

#[utoipa::path(
    post,
    path = "/faker/{id}/start",
    tag = "faker",
    summary = "Start a faker instance",
    description = "Starts ratio faking for the specified instance. Sends 'started' event to tracker. If the instance already exists, updates its config before starting.",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Instance ID to start")
    ),
    request_body = StartFakerRequest,
    responses(
        (status = 200, description = "Faker started", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Failed to start faker", body = ApiError)
    )
)]
pub async fn start_faker(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(request): Json<StartFakerRequest>,
) -> Response {
    if state.app.instance_exists(&id).await {
        if let Err(e) = state.app.update_instance_config(&id, request.config).await {
            return ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e);
        }
    } else if let Err(e) = state.app.create_instance(&id, request.torrent, request.config).await {
        return ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e);
    }

    match state.app.start_instance(&id).await {
        Ok(()) => ApiSuccess::response(EmptyData {}),
        Err(e) => ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

#[utoipa::path(
    post,
    path = "/faker/{id}/stop",
    tag = "faker",
    summary = "Stop a faker instance",
    description = "Stops ratio faking for the specified instance. Sends 'stopped' event to tracker and returns final stats.",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Instance ID to stop")
    ),
    responses(
        (status = 200, description = "Faker stopped, returns final stats", body = ApiSuccess<Object>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Instance not found", body = ApiError)
    )
)]
pub async fn stop_faker(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.stop_instance(&id).await {
        Ok(stats) => ApiSuccess::response(stats),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

#[utoipa::path(
    post,
    path = "/faker/{id}/pause",
    tag = "faker",
    summary = "Pause a faker instance",
    description = "Pauses ratio faking. The instance remains active but stops accumulating stats and announcing.",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Instance ID to pause")
    ),
    responses(
        (status = 200, description = "Faker paused", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Instance not found", body = ApiError)
    )
)]
pub async fn pause_faker(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.pause_instance(&id).await {
        Ok(()) => ApiSuccess::response(EmptyData {}),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

#[utoipa::path(
    post,
    path = "/faker/{id}/resume",
    tag = "faker",
    summary = "Resume a faker instance",
    description = "Resumes a paused faker instance. Continues accumulating stats and announcing to tracker.",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Instance ID to resume")
    ),
    responses(
        (status = 200, description = "Faker resumed", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Instance not found", body = ApiError)
    )
)]
pub async fn resume_faker(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.resume_instance(&id).await {
        Ok(()) => ApiSuccess::response(EmptyData {}),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

#[utoipa::path(
    post,
    path = "/faker/{id}/update",
    tag = "faker",
    summary = "Force tracker announce",
    description = "Forces an immediate tracker announce for the instance. Updates stats and returns current statistics.",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Instance ID to update")
    ),
    responses(
        (status = 200, description = "Announce sent, returns current stats", body = ApiSuccess<Object>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Instance not found", body = ApiError)
    )
)]
pub async fn update_faker(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.update_instance(&id).await {
        Ok(stats) => ApiSuccess::response(stats),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

#[utoipa::path(
    post,
    path = "/faker/{id}/stats-only",
    tag = "faker",
    summary = "Update stats only",
    description = "Updates and returns current statistics without sending a tracker announce.",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Instance ID")
    ),
    responses(
        (status = 200, description = "Stats updated", body = ApiSuccess<Object>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Instance not found", body = ApiError)
    )
)]
pub async fn update_stats_only(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.update_stats_only(&id).await {
        Ok(stats) => ApiSuccess::response(stats),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

#[utoipa::path(
    get,
    path = "/faker/{id}/stats",
    tag = "faker",
    summary = "Get instance statistics",
    description = "Returns the current statistics for a faker instance including upload/download amounts, ratio, and rates.",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Instance ID")
    ),
    responses(
        (status = 200, description = "Current statistics", body = ApiSuccess<Object>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Instance not found", body = ApiError)
    )
)]
pub async fn get_stats(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.get_stats(&id).await {
        Ok(stats) => ApiSuccess::response(stats),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/faker/{id}/start", post(start_faker))
        .route("/faker/{id}/stop", post(stop_faker))
        .route("/faker/{id}/pause", post(pause_faker))
        .route("/faker/{id}/resume", post(resume_faker))
        .route("/faker/{id}/update", post(update_faker))
        .route("/faker/{id}/stats", get(get_stats))
        .route("/faker/{id}/stats-only", post(update_stats_only))
}
