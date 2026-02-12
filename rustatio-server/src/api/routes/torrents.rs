//! Torrent file upload endpoint.

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::Response,
    routing::post,
    Router,
};
use rustatio_core::TorrentInfo;
use serde::Serialize;
use utoipa::ToSchema;

use crate::api::{
    common::{ApiError, ApiSuccess},
    ServerState,
};

#[derive(Serialize, ToSchema)]
pub struct LoadTorrentResponse {
    #[schema(value_type = Object)]
    pub torrent: TorrentInfo,
}

#[utoipa::path(
    post,
    path = "/torrent/load",
    tag = "torrents",
    summary = "Load a torrent file",
    description = "Uploads and parses a .torrent file. Returns the parsed torrent information.",
    security(("bearer_auth" = [])),
    request_body(content_type = "multipart/form-data", description = "Torrent file upload (field name: file)"),
    responses(
        (status = 200, description = "Torrent loaded successfully", body = ApiSuccess<LoadTorrentResponse>),
        (status = 400, description = "Invalid torrent file", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
pub async fn load_torrent(State(_state): State<ServerState>, mut multipart: Multipart) -> Response {
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            match field.bytes().await {
                Ok(bytes) => match TorrentInfo::from_bytes(&bytes) {
                    Ok(torrent) => {
                        return ApiSuccess::response(LoadTorrentResponse { torrent });
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

pub fn router() -> Router<ServerState> {
    Router::new().route("/torrent/load", post(load_torrent))
}
