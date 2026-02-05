//! Default configuration endpoints.

use axum::{
    extract::State,
    http::StatusCode,
    response::Response,
    routing::{delete, get, put},
    Json, Router,
};
use rustatio_core::{FakerConfig, PresetSettings};

use crate::api::{
    common::{ApiError, ApiSuccess, EmptyData},
    ServerState,
};

#[utoipa::path(
    get,
    path = "/config/default",
    tag = "config",
    summary = "Get default configuration",
    description = "Returns the default configuration used for new instances (e.g., from watch folder).",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Default configuration", body = ApiSuccess<Object>),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
pub async fn get_default_config(State(state): State<ServerState>) -> Response {
    let config = state.app.get_default_config().await;
    ApiSuccess::response(config)
}

#[utoipa::path(
    put,
    path = "/config/default",
    tag = "config",
    summary = "Set default configuration",
    description = "Sets the default configuration to be used for new instances.",
    security(("bearer_auth" = [])),
    request_body(content = Object, description = "Preset settings in UI-friendly format"),
    responses(
        (status = 200, description = "Configuration saved", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Failed to save configuration", body = ApiError)
    )
)]
pub async fn set_default_config(State(state): State<ServerState>, Json(preset): Json<PresetSettings>) -> Response {
    let config: FakerConfig = preset.into();
    match state.app.set_default_config(Some(config)).await {
        Ok(()) => ApiSuccess::response(EmptyData {}),
        Err(e) => ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

#[utoipa::path(
    delete,
    path = "/config/default",
    tag = "config",
    summary = "Clear default configuration",
    description = "Clears the custom default configuration, reverting to built-in defaults.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Configuration cleared", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Failed to clear configuration", body = ApiError)
    )
)]
pub async fn clear_default_config(State(state): State<ServerState>) -> Response {
    match state.app.set_default_config(None).await {
        Ok(()) => ApiSuccess::response(EmptyData {}),
        Err(e) => ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/config/default", get(get_default_config))
        .route("/config/default", put(set_default_config))
        .route("/config/default", delete(clear_default_config))
}
