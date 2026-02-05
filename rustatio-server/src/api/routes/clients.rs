//! BitTorrent client information endpoints.

use axum::{response::Response, routing::get, Router};
use rustatio_core::ClientType;
use serde::Serialize;
use utoipa::ToSchema;

use crate::api::{common::ApiSuccess, ServerState};

#[derive(Serialize, ToSchema)]
pub struct ClientInfoResponse {
    pub id: String,
    pub name: String,
    pub default_version: String,
    pub versions: Vec<String>,
    pub default_port: u16,
}

impl From<rustatio_core::ClientInfo> for ClientInfoResponse {
    fn from(info: rustatio_core::ClientInfo) -> Self {
        ClientInfoResponse {
            id: info.id,
            name: info.name,
            default_version: info.default_version,
            versions: info.versions,
            default_port: info.default_port,
        }
    }
}

#[utoipa::path(
    get,
    path = "/clients",
    tag = "clients",
    summary = "Get available client types",
    description = "Returns a list of BitTorrent client type IDs that can be emulated.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of client type IDs", body = ApiSuccess<Vec<String>>),
        (status = 401, description = "Unauthorized", body = crate::api::common::ApiError)
    )
)]
pub async fn get_client_types() -> Response {
    ApiSuccess::response(ClientType::all_ids())
}

#[utoipa::path(
    get,
    path = "/clients/info",
    tag = "clients",
    summary = "Get detailed client information",
    description = "Returns detailed information about all available BitTorrent clients including versions and default ports.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of client information", body = ApiSuccess<Vec<ClientInfoResponse>>),
        (status = 401, description = "Unauthorized", body = crate::api::common::ApiError)
    )
)]
pub async fn get_client_infos() -> Response {
    let infos: Vec<ClientInfoResponse> = ClientType::all_infos().into_iter().map(|i| i.into()).collect();
    ApiSuccess::response(infos)
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/clients", get(get_client_types))
        .route("/clients/info", get(get_client_infos))
}
