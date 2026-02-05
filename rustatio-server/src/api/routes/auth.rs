//! Authentication endpoints.

use axum::{response::Response, routing::get, Router};
use serde::Serialize;
use utoipa::ToSchema;

use crate::api::{
    common::{ApiSuccess, EmptyData},
    middleware, ServerState,
};

#[derive(Serialize, ToSchema)]
pub struct AuthStatusResponse {
    pub auth_enabled: bool,
}

/// Check if authentication is enabled (no auth required)
#[utoipa::path(
    get,
    path = "/auth/status",
    tag = "auth",
    summary = "Check authentication status",
    description = "Returns whether authentication is enabled on the server. This endpoint does not require authentication.",
    responses(
        (status = 200, description = "Auth status retrieved", body = ApiSuccess<AuthStatusResponse>)
    )
)]
pub async fn auth_status() -> Response {
    ApiSuccess::response(AuthStatusResponse {
        auth_enabled: middleware::is_auth_enabled(),
    })
}

/// Verify authentication token
#[utoipa::path(
    get,
    path = "/auth/verify",
    tag = "auth",
    summary = "Verify authentication token",
    description = "Verifies the provided authentication token. Returns success if the token is valid.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Token is valid", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Invalid or missing token", body = crate::api::common::ApiError)
    )
)]
pub async fn verify_auth() -> Response {
    ApiSuccess::response(EmptyData {})
}

pub fn public_router() -> Router<ServerState> {
    Router::new().route("/auth/status", get(auth_status))
}

pub fn router() -> Router<ServerState> {
    Router::new().route("/auth/verify", get(verify_auth))
}
