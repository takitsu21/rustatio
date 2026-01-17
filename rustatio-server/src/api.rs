use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{delete, get, patch, post},
    Json, Router,
};
use futures::stream::Stream;
use rustatio_core::{FakerConfig, TorrentInfo};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::auth;
use crate::state::InstanceInfo;
use crate::watch::{WatchStatus, WatchedFile};
use crate::ServerState;

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
pub fn router() -> Router<ServerState> {
    Router::new()
        // Instance management
        .route("/instances", get(list_instances).post(create_instance))
        .route("/instances/{id}", delete(delete_instance))
        .route("/instances/{id}/torrent", post(load_instance_torrent))
        .route("/instances/{id}/config", patch(update_instance_config))
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
        // SSE streaming
        .route("/logs", get(logs_sse))
        .route("/events", get(instances_sse))
        // Watch folder
        .route("/watch/status", get(get_watch_status))
        .route("/watch/files", get(list_watch_files))
        .route("/watch/files/{filename}", delete(delete_watch_file))
        // Auth verification (returns success if token is valid)
        .route("/auth/verify", get(verify_auth))
}

/// Auth-free router for endpoints that don't require authentication
pub fn public_router() -> Router<ServerState> {
    Router::new()
        // Auth status check (no auth required - tells UI if auth is enabled)
        .route("/auth/status", get(auth_status))
}

// =============================================================================
// Auth Endpoints
// =============================================================================

/// Auth status response
#[derive(Serialize)]
struct AuthStatusResponse {
    auth_enabled: bool,
}

/// Check if authentication is enabled (no auth required for this endpoint)
async fn auth_status() -> Response {
    ApiSuccess::response(AuthStatusResponse {
        auth_enabled: auth::is_auth_enabled(),
    })
}

/// Verify authentication token (if this returns success, the token is valid)
async fn verify_auth() -> Response {
    // If we reach here, the auth middleware already validated the token
    ApiSuccess::response(())
}

/// Create a new instance ID
#[derive(Serialize)]
struct CreateInstanceResponse {
    id: String,
}

async fn create_instance(State(state): State<ServerState>) -> Response {
    let id = state.app.next_instance_id().await;
    ApiSuccess::response(CreateInstanceResponse { id })
}

/// List all instances with their current stats
async fn list_instances(State(state): State<ServerState>) -> Response {
    let instances: Vec<InstanceInfo> = state.app.list_instances().await;
    ApiSuccess::response(instances)
}

/// Query parameters for delete instance
#[derive(Deserialize)]
struct DeleteInstanceQuery {
    #[serde(default)]
    force: bool,
}

/// Delete an instance
async fn delete_instance(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Query(query): Query<DeleteInstanceQuery>,
) -> Response {
    match state.app.delete_instance(&id, query.force).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::BAD_REQUEST, e),
    }
}

/// Load torrent response
#[derive(Serialize)]
struct LoadTorrentResponse {
    torrent_id: String,
    torrent: TorrentInfo,
}

/// Load a torrent file
async fn load_torrent(State(state): State<ServerState>, mut multipart: Multipart) -> Response {
    // Extract the torrent file from multipart form data
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            match field.bytes().await {
                Ok(bytes) => match TorrentInfo::from_bytes(&bytes) {
                    Ok(torrent) => {
                        // Generate a temporary ID and store the torrent
                        let torrent_id = uuid::Uuid::new_v4().to_string();
                        let torrent_data = torrent.clone();
                        state.app.store_torrent(&torrent_id, torrent).await;

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

/// Load a torrent file for a specific instance (creates idle instance on server)
/// This allows the instance to persist across page refreshes
async fn load_instance_torrent(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Response {
    // Extract the torrent file from multipart form data
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            match field.bytes().await {
                Ok(bytes) => match TorrentInfo::from_bytes(&bytes) {
                    Ok(torrent) => {
                        // Check if instance already exists
                        if state.app.instance_exists(&id).await {
                            // Update existing instance with new torrent
                            // For now, we just return success - the torrent is already parsed
                            // The frontend will handle updating its state
                            return ApiSuccess::response(LoadTorrentResponse {
                                torrent_id: id,
                                torrent,
                            });
                        }

                        // Create idle instance on server (will persist across refreshes)
                        if let Err(e) = state.app.create_idle_instance(&id, torrent.clone()).await {
                            return ApiError::response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                format!("Failed to create instance: {}", e),
                            );
                        }

                        return ApiSuccess::response(LoadTorrentResponse {
                            torrent_id: id,
                            torrent,
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

/// Update instance config (without starting the faker)
/// Used to persist form changes before the faker is started
async fn update_instance_config(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(config): Json<FakerConfig>,
) -> Response {
    match state.app.update_instance_config_only(&id, config).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Request body for starting a faker
#[derive(Deserialize)]
struct StartFakerRequest {
    torrent: TorrentInfo,
    config: FakerConfig,
}

/// Start a faker instance
///
/// If the instance already exists (e.g., from watch folder), it will update the config
/// and start it. Otherwise, it creates a new instance with the provided torrent and config.
async fn start_faker(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(request): Json<StartFakerRequest>,
) -> Response {
    // Check if instance already exists (e.g., from watch folder)
    if state.app.instance_exists(&id).await {
        // Update config for existing instance
        if let Err(e) = state.app.update_instance_config(&id, request.config).await {
            return ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e);
        }
    } else {
        // Create new instance with provided torrent and config
        if let Err(e) = state.app.create_instance(&id, request.torrent, request.config).await {
            return ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e);
        }
    }

    // Start the faker
    match state.app.start_instance(&id).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

/// Stop a faker instance
async fn stop_faker(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.stop_instance(&id).await {
        Ok(stats) => ApiSuccess::response(stats),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Pause a faker instance
async fn pause_faker(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.pause_instance(&id).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Resume a faker instance
async fn resume_faker(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.resume_instance(&id).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Update a faker instance (send tracker announce)
async fn update_faker(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.update_instance(&id).await {
        Ok(stats) => ApiSuccess::response(stats),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Update stats only (no tracker announce)
async fn update_stats_only(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.update_stats_only(&id).await {
        Ok(stats) => ApiSuccess::response(stats),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Get stats for a faker instance
async fn get_stats(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.get_stats(&id).await {
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

/// Response from ip-api.com (fallback)
#[derive(Deserialize)]
struct IpApiResponse {
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    city: Option<String>,
    #[serde(default)]
    country: Option<String>,
    #[serde(default, rename = "countryCode")]
    country_code: Option<String>,
    #[serde(default)]
    isp: Option<String>,
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
/// Tries multiple IP lookup services with fallbacks for reliability
async fn get_network_status() -> Response {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
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

    // Try ip-api.com first (more reliable, no rate limits for non-commercial use)
    if let Ok(response) = client.get("http://ip-api.com/json").send().await {
        if let Ok(ip_info) = response.json::<IpApiResponse>().await {
            if let Some(ip) = ip_info.query {
                // Use ISP or org for VPN detection
                let org = ip_info.isp.or(ip_info.org);
                let vpn_provider = detect_vpn_provider(&org);
                let is_vpn = vpn_provider.is_some();

                let status = NetworkStatus {
                    ip,
                    country: ip_info.country_code.or(ip_info.country),
                    city: ip_info.city,
                    org,
                    is_vpn,
                    vpn_provider,
                };

                return ApiSuccess::response(status);
            }
        }
    }

    // Fallback to ipinfo.io
    if let Ok(response) = client.get("https://ipinfo.io/json").send().await {
        if let Ok(ip_info) = response.json::<IpInfoResponse>().await {
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

            return ApiSuccess::response(status);
        }
    }

    // Last resort: just get the IP from ipify
    if let Ok(response) = client.get("https://api.ipify.org?format=json").send().await {
        #[derive(Deserialize)]
        struct IpifyResponse {
            ip: String,
        }

        if let Ok(ip_info) = response.json::<IpifyResponse>().await {
            let status = NetworkStatus {
                ip: ip_info.ip,
                country: None,
                city: None,
                org: None,
                is_vpn: false,
                vpn_provider: None,
            };

            return ApiSuccess::response(status);
        }
    }

    ApiError::response(
        StatusCode::BAD_GATEWAY,
        "Failed to fetch IP info from all providers".to_string(),
    )
}

/// SSE endpoint for streaming logs to the UI
async fn logs_sse(State(state): State<ServerState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.app.subscribe_logs();

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

/// SSE endpoint for streaming instance events to the UI (for real-time sync)
async fn instances_sse(State(state): State<ServerState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.app.subscribe_instance_events();

    let stream = BroadcastStream::new(rx).filter_map(|result| {
        result.ok().map(|instance_event| {
            Ok(Event::default()
                .event("instance")
                .json_data(&instance_event)
                .unwrap_or_else(|_| Event::default()))
        })
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// =============================================================================
// Watch Folder Endpoints
// =============================================================================

/// Get watch folder status
async fn get_watch_status(State(state): State<ServerState>) -> Response {
    let watch = state.watch.read().await;
    let status: WatchStatus = watch.get_status().await;
    ApiSuccess::response(status)
}

/// List all torrent files in watch folder
async fn list_watch_files(State(state): State<ServerState>) -> Response {
    let watch = state.watch.read().await;
    let files: Vec<WatchedFile> = watch.list_files().await;
    ApiSuccess::response(files)
}

/// Delete a torrent file from watch folder
async fn delete_watch_file(State(state): State<ServerState>, Path(filename): Path<String>) -> Response {
    let watch = state.watch.read().await;
    match watch.delete_file(&filename).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}
