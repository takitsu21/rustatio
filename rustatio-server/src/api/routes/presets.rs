use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    routing::{delete, get, put},
    Json, Router,
};

use crate::api::{
    common::{ApiError, ApiSuccess, EmptyData},
    ServerState,
};
use crate::services::persistence::CustomPreset;

#[utoipa::path(
    get,
    path = "/presets/custom",
    tag = "config",
    summary = "List custom presets",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Custom presets", body = ApiSuccess<Vec<Object>>),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
pub async fn list_custom_presets(State(state): State<ServerState>) -> Response {
    ApiSuccess::response(state.app.list_custom_presets().await)
}

#[utoipa::path(
    put,
    path = "/presets/custom/{id}",
    tag = "config",
    summary = "Create or update custom preset",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Preset ID")),
    request_body(content = Object, description = "Custom preset"),
    responses(
        (status = 200, description = "Preset saved", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Failed to save preset", body = ApiError)
    )
)]
pub async fn upsert_custom_preset(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(mut preset): Json<CustomPreset>,
) -> Response {
    preset.id = id;

    match state.app.upsert_custom_preset(preset).await {
        Ok(()) => ApiSuccess::response(EmptyData {}),
        Err(e) => ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

#[utoipa::path(
    delete,
    path = "/presets/custom/{id}",
    tag = "config",
    summary = "Delete custom preset",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Preset ID")),
    responses(
        (status = 200, description = "Preset deleted", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Failed to delete preset", body = ApiError)
    )
)]
pub async fn delete_custom_preset(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Response {
    match state.app.delete_custom_preset(&id).await {
        Ok(()) => ApiSuccess::response(EmptyData {}),
        Err(e) => ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/presets/custom", get(list_custom_presets))
        .route("/presets/custom/{id}", put(upsert_custom_preset))
        .route("/presets/custom/{id}", delete(delete_custom_preset))
}
