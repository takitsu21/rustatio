use axum::{extract::Query, http::StatusCode, response::Response, routing::get, Router};
use serde::{Deserialize, Serialize};

use crate::api::{
    common::{ApiError, ApiSuccess},
    ServerState,
};

#[derive(Deserialize)]
pub struct BrowseQuery {
    #[serde(default = "default_path")]
    pub path: String,
}

fn default_path() -> String {
    "/".to_string()
}

#[derive(Serialize)]
pub struct BrowseEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
}

#[derive(Serialize)]
pub struct BrowseResponse {
    pub path: String,
    pub parent: Option<String>,
    pub entries: Vec<BrowseEntry>,
}

pub async fn browse_directory(Query(query): Query<BrowseQuery>) -> Response {
    let path = std::path::Path::new(&query.path);

    if !path.exists() {
        return ApiError::response(StatusCode::NOT_FOUND, format!("Path not found: {}", query.path));
    }

    if !path.is_dir() {
        return ApiError::response(StatusCode::BAD_REQUEST, format!("Not a directory: {}", query.path));
    }

    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            return ApiError::response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to resolve path: {}", e),
            );
        }
    };

    let parent = canonical.parent().map(|p| p.to_string_lossy().to_string());

    let entries = match std::fs::read_dir(&canonical) {
        Ok(entries) => entries,
        Err(e) => {
            return ApiError::response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read directory: {}", e),
            );
        }
    };

    let mut result: Vec<BrowseEntry> = Vec::new();

    for entry in entries.flatten() {
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files/dirs
        if name.starts_with('.') {
            continue;
        }

        let entry_path = entry.path().to_string_lossy().to_string();
        let is_dir = file_type.is_dir();

        let size = if !is_dir {
            entry.metadata().ok().map(|m| m.len())
        } else {
            None
        };

        // Only show directories and .torrent files
        if is_dir || name.ends_with(".torrent") {
            result.push(BrowseEntry {
                name,
                path: entry_path,
                is_dir,
                size,
            });
        }
    }

    // Sort: directories first, then files, both alphabetically
    result.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    ApiSuccess::response(BrowseResponse {
        path: canonical.to_string_lossy().to_string(),
        parent,
        entries: result,
    })
}

pub fn router() -> Router<ServerState> {
    Router::new().route("/browse", get(browse_directory))
}
