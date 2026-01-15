use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{delete, get, post},
    Json, Router,
};
use futures::stream::Stream;
use rustatio_core::{FakerConfig, TorrentInfo};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::state::AppState;

/// API error response
#[derive(Serialize)]
struct ApiError {
    success: bool,
    error: String,
}

impl ApiError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: message.into(),
        }
    }

    fn response(status: StatusCode, message: impl Into<String>) -> Response {
        (status, Json(Self::new(message))).into_response()
    }
}

/// API success response
#[derive(Serialize)]
struct ApiSuccess<T> {
    success: bool,
    data: T,
}

impl<T: Serialize> ApiSuccess<T> {
    fn new(data: T) -> Self {
        Self { success: true, data }
    }

    fn response(data: T) -> Response
    where
        T: Serialize,
    {
        (StatusCode::OK, Json(Self::new(data))).into_response()
    }
}

/// Build the API router
pub fn router() -> Router<AppState> {
    Router::new()
        // Instance management
        .route("/instances", post(create_instance))
        .route("/instances/{id}", delete(delete_instance))
        // Torrent loading
        .route("/torrent/load", post(load_torrent))
        // Faker operations
        .route("/faker/{id}/start", post(start_faker))
        .route("/faker/{id}/stop", post(stop_faker))
        .route("/faker/{id}/pause", post(pause_faker))
        .route("/faker/{id}/resume", post(resume_faker))
        .route("/faker/{id}/update", post(update_faker))
        .route("/faker/{id}/stats", get(get_stats))
        .route("/faker/{id}/stats-only", post(update_stats_only))
        // Client types
        .route("/clients", get(get_client_types))
        // Network status (VPN detection)
        .route("/network/status", get(get_network_status))
        // Log streaming (SSE)
        .route("/logs", get(logs_sse))
}

/// Create a new instance ID
#[derive(Serialize)]
struct CreateInstanceResponse {
    id: String,
}

async fn create_instance(State(state): State<AppState>) -> Response {
    let id = state.next_instance_id().await;
    ApiSuccess::response(CreateInstanceResponse { id })
}

/// Delete an instance
async fn delete_instance(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.delete_instance(&id).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Load torrent response
#[derive(Serialize)]
struct LoadTorrentResponse {
    torrent_id: String,
    torrent: TorrentInfo,
}

/// Load a torrent file
async fn load_torrent(State(state): State<AppState>, mut multipart: Multipart) -> Response {
    // Extract the torrent file from multipart form data
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            match field.bytes().await {
                Ok(bytes) => match TorrentInfo::from_bytes(&bytes) {
                    Ok(torrent) => {
                        // Generate a temporary ID and store the torrent
                        let torrent_id = uuid::Uuid::new_v4().to_string();
                        let torrent_data = torrent.clone();
                        state.store_torrent(&torrent_id, torrent).await;

                        return ApiSuccess::response(LoadTorrentResponse {
                            torrent_id,
                            torrent: torrent_data,
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

/// Request body for starting a faker
#[derive(Deserialize)]
struct StartFakerRequest {
    torrent: TorrentInfo,
    config: FakerConfig,
}

/// Start a faker instance
async fn start_faker(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<StartFakerRequest>,
) -> Response {
    // Create the instance with the provided torrent and config
    if let Err(e) = state.create_instance(&id, request.torrent, request.config).await {
        return ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e);
    }

    // Start the faker
    match state.start_instance(&id).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

/// Stop a faker instance
async fn stop_faker(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.stop_instance(&id).await {
        Ok(stats) => ApiSuccess::response(stats),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Pause a faker instance
async fn pause_faker(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.pause_instance(&id).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Resume a faker instance
async fn resume_faker(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.resume_instance(&id).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Update a faker instance (send tracker announce)
async fn update_faker(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.update_instance(&id).await {
        Ok(stats) => ApiSuccess::response(stats),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Update stats only (no tracker announce)
async fn update_stats_only(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.update_stats_only(&id).await {
        Ok(stats) => ApiSuccess::response(stats),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Get stats for a faker instance
async fn get_stats(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.get_stats(&id).await {
        Ok(stats) => ApiSuccess::response(stats),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Get available client types
async fn get_client_types() -> Response {
    let types = vec!["utorrent", "qbittorrent", "transmission", "deluge"];
    ApiSuccess::response(types)
}

/// Network status response
#[derive(Serialize)]
struct NetworkStatus {
    ip: String,
    country: Option<String>,
    city: Option<String>,
    org: Option<String>,
    is_vpn: bool,
    vpn_provider: Option<String>,
}

/// Response from ipinfo.io
#[derive(Deserialize)]
struct IpInfoResponse {
    ip: String,
    #[serde(default)]
    city: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    region: Option<String>,
    #[serde(default)]
    country: Option<String>,
    #[serde(default)]
    org: Option<String>,
}

/// Known VPN provider patterns to detect
const VPN_PROVIDERS: &[(&str, &str)] = &[
    ("proton", "ProtonVPN"),
    ("mullvad", "Mullvad"),
    ("nordvpn", "NordVPN"),
    ("nord", "NordVPN"),
    ("expressvpn", "ExpressVPN"),
    ("express", "ExpressVPN"),
    ("surfshark", "Surfshark"),
    ("private internet access", "Private Internet Access"),
    ("pia", "Private Internet Access"),
    ("windscribe", "Windscribe"),
    ("cyberghost", "CyberGhost"),
    ("ipvanish", "IPVanish"),
    ("tunnelbear", "TunnelBear"),
    ("hotspot shield", "Hotspot Shield"),
    ("vyprvpn", "VyprVPN"),
    ("hide.me", "Hide.me"),
    ("perfect privacy", "Perfect Privacy"),
    ("airvpn", "AirVPN"),
    ("privatevpn", "PrivateVPN"),
    ("torguard", "TorGuard"),
    ("ivpn", "IVPN"),
    ("ovpn", "OVPN"),
    ("m247", "M247 (VPN Infrastructure)"),
    ("datacamp", "Datacamp (VPN/Proxy)"),
    ("hostwinds", "Hostwinds (VPN/VPS)"),
    ("choopa", "Choopa/Vultr (VPN/VPS)"),
    ("linode", "Linode (VPN/VPS)"),
    ("digitalocean", "DigitalOcean (VPN/VPS)"),
];

/// Detect VPN provider from organization string
fn detect_vpn_provider(org: &Option<String>) -> Option<String> {
    let org_lower = org.as_ref()?.to_lowercase();

    for (pattern, provider) in VPN_PROVIDERS {
        if org_lower.contains(pattern) {
            return Some(provider.to_string());
        }
    }

    None
}

/// Get network status (public IP and VPN detection)
async fn get_network_status() -> Response {
    // Fetch IP info from ipinfo.io
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return ApiError::response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create HTTP client: {}", e),
            );
        }
    };

    let response = match client.get("https://ipinfo.io/json").send().await {
        Ok(r) => r,
        Err(e) => {
            return ApiError::response(StatusCode::BAD_GATEWAY, format!("Failed to fetch IP info: {}", e));
        }
    };

    let ip_info: IpInfoResponse = match response.json().await {
        Ok(info) => info,
        Err(e) => {
            return ApiError::response(
                StatusCode::BAD_GATEWAY,
                format!("Failed to parse IP info response: {}", e),
            );
        }
    };

    // Detect VPN from organization
    let vpn_provider = detect_vpn_provider(&ip_info.org);
    let is_vpn = vpn_provider.is_some();

    let status = NetworkStatus {
        ip: ip_info.ip,
        country: ip_info.country,
        city: ip_info.city,
        org: ip_info.org,
        is_vpn,
        vpn_provider,
    };

    ApiSuccess::response(status)
}

/// SSE endpoint for streaming logs to the UI
async fn logs_sse(State(state): State<AppState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.subscribe_logs();

    let stream = BroadcastStream::new(rx).filter_map(|result| {
        result.ok().map(|log_event| {
            Ok(Event::default()
                .event("log")
                .json_data(&log_event)
                .unwrap_or_else(|_| Event::default()))
        })
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
