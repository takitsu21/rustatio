use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use futures::stream::Stream;
use rustatio_core::{FakerConfig, PresetSettings, TorrentInfo};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi, ToSchema};

use crate::auth;
use crate::persistence::InstanceSource;
use crate::state::{InstanceEvent, InstanceInfo, LogEvent};
use crate::watch::{WatchStatus, WatchedFile, WatchedFileStatus};
use crate::ServerState;

/// Security scheme modifier to add Bearer token authentication
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("token")
                        .description(Some(
                            "Enter your AUTH_TOKEN value. This is the same token set in the AUTH_TOKEN environment variable.",
                        ))
                        .build(),
                ),
            );
        }
    }
}

/// OpenAPI documentation for the Rustatio API
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Rustatio API",
        version = env!("CARGO_PKG_VERSION"),
        description = "HTTP API for Rustatio - ratio faking server for BitTorrent trackers",
        license(name = "MIT")
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "instances", description = "Faker instance management"),
        (name = "faker", description = "Faker control operations"),
        (name = "torrents", description = "Torrent file handling"),
        (name = "clients", description = "BitTorrent client emulation"),
        (name = "network", description = "Network and VPN status"),
        (name = "watch", description = "Watch folder management"),
        (name = "config", description = "Default configuration"),
        (name = "events", description = "Server-Sent Events streams")
    ),
    paths(
        auth_status,
        verify_auth,
        list_instances,
        create_instance,
        delete_instance,
        load_torrent,
        load_instance_torrent,
        update_instance_config,
        start_faker,
        stop_faker,
        pause_faker,
        resume_faker,
        update_faker,
        update_stats_only,
        get_stats,
        get_client_types,
        get_network_status,
        logs_sse,
        instances_sse,
        get_watch_status,
        list_watch_files,
        delete_watch_file,
        reload_watch_file,
        reload_all_watch_files,
        get_default_config,
        set_default_config,
        clear_default_config
    ),
    components(
        schemas(
            ApiError,
            ApiSuccess<EmptyData>,
            EmptyData,
            AuthStatusResponse,
            CreateInstanceResponse,
            DeleteInstanceQuery,
            LoadTorrentResponse,
            StartFakerRequest,
            NetworkStatus,
            ReloadAllResponse,
            InstanceInfo,
            InstanceSource,
            WatchStatus,
            WatchedFile,
            WatchedFileStatus,
            LogEvent,
            InstanceEvent
        )
    ),
    modifiers(&SecurityAddon),
    servers(
        (url = "/api", description = "API base path")
    )
)]
pub struct ApiDoc;

/// API error response
#[derive(Serialize, ToSchema)]
pub struct ApiError {
    /// Always false for error responses
    success: bool,
    /// Error message describing what went wrong
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

/// API success response wrapper
#[derive(Serialize, ToSchema)]
pub struct ApiSuccess<T> {
    /// Always true for success responses
    success: bool,
    /// Response data
    data: T,
}

/// Empty data placeholder for responses that don't return any data
#[derive(Serialize, ToSchema)]
pub struct EmptyData {}

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
        .route("/watch/files/{filename}/reload", post(reload_watch_file))
        .route("/watch/reload", post(reload_all_watch_files))
        // Default config for new instances
        .route("/config/default", get(get_default_config))
        .route("/config/default", put(set_default_config))
        .route("/config/default", delete(clear_default_config))
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
#[derive(Serialize, ToSchema)]
struct AuthStatusResponse {
    /// Whether authentication is enabled on the server
    auth_enabled: bool,
}

/// Check if authentication is enabled (no auth required for this endpoint)
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
async fn auth_status() -> Response {
    ApiSuccess::response(AuthStatusResponse {
        auth_enabled: auth::is_auth_enabled(),
    })
}

/// Verify authentication token (if this returns success, the token is valid)
#[utoipa::path(
    get,
    path = "/auth/verify",
    tag = "auth",
    summary = "Verify authentication token",
    description = "Verifies the provided authentication token. Returns success if the token is valid.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Token is valid", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Invalid or missing token", body = ApiError)
    )
)]
async fn verify_auth() -> Response {
    // If we reach here, the auth middleware already validated the token
    ApiSuccess::response(())
}

/// Create a new instance ID
#[derive(Serialize, ToSchema)]
struct CreateInstanceResponse {
    /// Generated unique instance identifier
    id: String,
}

/// Create a new faker instance
#[utoipa::path(
    post,
    path = "/instances",
    tag = "instances",
    summary = "Create a new instance",
    description = "Generates a new unique instance ID. The instance is not started until you call the start endpoint.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Instance ID created", body = ApiSuccess<CreateInstanceResponse>),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
async fn create_instance(State(state): State<ServerState>) -> Response {
    let id = state.app.next_instance_id().await;
    ApiSuccess::response(CreateInstanceResponse { id })
}

/// List all instances with their current stats
#[utoipa::path(
    get,
    path = "/instances",
    tag = "instances",
    summary = "List all instances",
    description = "Returns a list of all faker instances with their current statistics and configuration.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of instances", body = ApiSuccess<Vec<InstanceInfo>>),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
async fn list_instances(State(state): State<ServerState>) -> Response {
    let instances: Vec<InstanceInfo> = state.app.list_instances().await;
    ApiSuccess::response(instances)
}

/// Query parameters for delete instance
#[derive(Deserialize, ToSchema)]
struct DeleteInstanceQuery {
    /// Force delete even for watch folder instances
    #[serde(default)]
    force: bool,
}

/// Delete an instance
#[utoipa::path(
    delete,
    path = "/instances/{id}",
    tag = "instances",
    summary = "Delete an instance",
    description = "Deletes a faker instance. Watch folder instances require force=true to delete.",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Instance ID to delete"),
        ("force" = Option<bool>, Query, description = "Force delete even for watch folder instances")
    ),
    responses(
        (status = 200, description = "Instance deleted", body = ApiSuccess<EmptyData>),
        (status = 400, description = "Cannot delete watch folder instance without force", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
async fn delete_instance(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Query(query): Query<DeleteInstanceQuery>,
) -> Response {
    // Get instance info before deleting (to clear watch service if needed)
    let instance_info = state.app.get_instance_info_for_delete(&id).await;

    match state.app.delete_instance(&id, query.force).await {
        Ok(()) => {
            // If force deleting a watch folder instance, clear the info_hash
            // so the torrent can be reloaded from the watch folder
            if query.force {
                if let Some((source, info_hash)) = instance_info {
                    if source == InstanceSource::WatchFolder {
                        state.watch.write().await.remove_info_hash(&info_hash).await;
                    }
                }
            }
            ApiSuccess::response(())
        }
        Err(e) => ApiError::response(StatusCode::BAD_REQUEST, e),
    }
}

/// Load torrent response
#[derive(Serialize, ToSchema)]
struct LoadTorrentResponse {
    /// Unique identifier for the loaded torrent
    torrent_id: String,
    /// Parsed torrent information
    #[schema(value_type = Object)]
    torrent: TorrentInfo,
}

/// Load a torrent file
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
#[utoipa::path(
    post,
    path = "/instances/{id}/torrent",
    tag = "instances",
    summary = "Load torrent for instance",
    description = "Uploads a torrent file and associates it with the specified instance ID. Creates an idle instance that persists across page refreshes.",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Instance ID to associate with the torrent")
    ),
    request_body(content_type = "multipart/form-data", description = "Torrent file upload (field name: file)"),
    responses(
        (status = 200, description = "Torrent loaded and instance created", body = ApiSuccess<LoadTorrentResponse>),
        (status = 400, description = "Invalid torrent file", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Failed to create instance", body = ApiError)
    )
)]
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
#[utoipa::path(
    patch,
    path = "/instances/{id}/config",
    tag = "instances",
    summary = "Update instance configuration",
    description = "Updates the configuration for an existing instance without starting it. Used to persist form changes.",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Instance ID to update")
    ),
    request_body(content = Object, description = "Faker configuration settings"),
    responses(
        (status = 200, description = "Configuration updated", body = ApiSuccess<EmptyData>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Instance not found", body = ApiError)
    )
)]
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
#[derive(Deserialize, ToSchema)]
struct StartFakerRequest {
    /// Torrent information
    #[schema(value_type = Object)]
    torrent: TorrentInfo,
    /// Faker configuration settings
    #[schema(value_type = Object)]
    config: FakerConfig,
}

/// Start a faker instance
///
/// If the instance already exists (e.g., from watch folder), it will update the config
/// and start it. Otherwise, it creates a new instance with the provided torrent and config.
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
async fn stop_faker(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.stop_instance(&id).await {
        Ok(stats) => ApiSuccess::response(stats),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Pause a faker instance
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
async fn pause_faker(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.pause_instance(&id).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Resume a faker instance
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
async fn resume_faker(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.resume_instance(&id).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Update a faker instance (send tracker announce)
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
async fn update_faker(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.update_instance(&id).await {
        Ok(stats) => ApiSuccess::response(stats),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Update stats only (no tracker announce)
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
async fn update_stats_only(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.update_stats_only(&id).await {
        Ok(stats) => ApiSuccess::response(stats),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Get stats for a faker instance
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
async fn get_stats(State(state): State<ServerState>, Path(id): Path<String>) -> Response {
    match state.app.get_stats(&id).await {
        Ok(stats) => ApiSuccess::response(stats),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Get available client types
#[utoipa::path(
    get,
    path = "/clients",
    tag = "clients",
    summary = "Get available client types",
    description = "Returns a list of BitTorrent client types that can be emulated.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of client types", body = ApiSuccess<Vec<String>>),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
async fn get_client_types() -> Response {
    let types = vec!["utorrent", "qbittorrent", "transmission", "deluge"];
    ApiSuccess::response(types)
}

/// Network status response from gluetun
#[derive(Serialize, ToSchema)]
struct NetworkStatus {
    /// Current public IP address
    ip: String,
    /// Country of the IP (if detected)
    country: Option<String>,
    /// ISP or organization name
    organization: Option<String>,
    /// Whether VPN is active and connected
    is_vpn: bool,
}

/// Response from gluetun control server /v1/vpn/status
#[derive(Deserialize)]
struct GluetunVpnStatus {
    status: String,
}

/// Response from gluetun control server /v1/publicip/ip
#[derive(Deserialize)]
struct GluetunPublicIp {
    public_ip: String,
    country: Option<String>,
    organization: Option<String>,
}

/// Get network status (public IP and VPN detection)
/// Uses gluetun's control server for definitive VPN detection.
/// This endpoint is only available when running with Docker + gluetun.
#[utoipa::path(
    get,
    path = "/network/status",
    tag = "network",
    summary = "Get network status",
    description = "Returns the current public IP address and VPN connection status. Requires gluetun container to be running.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Network status retrieved", body = ApiSuccess<NetworkStatus>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 503, description = "Gluetun not available", body = ApiError)
    )
)]
async fn get_network_status() -> Response {
    match try_gluetun_detection().await {
        Some(status) => ApiSuccess::response(status),
        None => ApiError::response(
            StatusCode::SERVICE_UNAVAILABLE,
            "Gluetun not available. Network status requires Docker with gluetun VPN container.",
        ),
    }
}

/// Try to detect VPN status via gluetun's control server
async fn try_gluetun_detection() -> Option<NetworkStatus> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1000))
        .build()
        .ok()?;

    // Get VPN status
    let vpn_status = client
        .get("http://localhost:8000/v1/vpn/status")
        .send()
        .await
        .ok()?
        .json::<GluetunVpnStatus>()
        .await
        .ok()?;

    let is_vpn = vpn_status.status == "running";

    // Get public IP (includes country and organization from geolocation)
    let public_ip = client
        .get("http://localhost:8000/v1/publicip/ip")
        .send()
        .await
        .ok()?
        .json::<GluetunPublicIp>()
        .await
        .ok()?;

    Some(NetworkStatus {
        ip: public_ip.public_ip,
        country: public_ip.country,
        organization: public_ip.organization,
        is_vpn,
    })
}

/// SSE endpoint for streaming logs to the UI
#[utoipa::path(
    get,
    path = "/logs",
    tag = "events",
    summary = "Stream logs via SSE",
    description = "Server-Sent Events stream for real-time log messages. Events are of type 'log' with LogEvent data.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "SSE stream established", content_type = "text/event-stream"),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
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
#[utoipa::path(
    get,
    path = "/events",
    tag = "events",
    summary = "Stream instance events via SSE",
    description = "Server-Sent Events stream for real-time instance updates. Events are of type 'instance' with InstanceEvent data (created/deleted).",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "SSE stream established", content_type = "text/event-stream"),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
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
async fn get_watch_status(State(state): State<ServerState>) -> Response {
    let watch = state.watch.read().await;
    let status: WatchStatus = watch.get_status().await;
    ApiSuccess::response(status)
}

/// List all torrent files in watch folder
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
async fn list_watch_files(State(state): State<ServerState>) -> Response {
    let watch = state.watch.read().await;
    let files: Vec<WatchedFile> = watch.list_files().await;
    ApiSuccess::response(files)
}

/// Delete a torrent file from watch folder
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
async fn delete_watch_file(State(state): State<ServerState>, Path(filename): Path<String>) -> Response {
    let watch = state.watch.read().await;
    match watch.delete_file(&filename).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::NOT_FOUND, e),
    }
}

/// Reload a single torrent file from watch folder
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
async fn reload_watch_file(State(state): State<ServerState>, Path(filename): Path<String>) -> Response {
    let watch = state.watch.read().await;
    match watch.reload_file(&filename).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

/// Reload all response
#[derive(Serialize, ToSchema)]
struct ReloadAllResponse {
    /// Number of torrent files reloaded
    reloaded: u32,
}

/// Reload all torrent files from watch folder (loads any new/unloaded files)
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
async fn reload_all_watch_files(State(state): State<ServerState>) -> Response {
    let watch = state.watch.read().await;
    match watch.reload_all().await {
        Ok(count) => ApiSuccess::response(ReloadAllResponse { reloaded: count }),
        Err(e) => ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

/// Get the default config for new instances
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
async fn get_default_config(State(state): State<ServerState>) -> Response {
    let config = state.app.get_default_config().await;
    ApiSuccess::response(config)
}

/// Set the default config for new instances
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
async fn set_default_config(State(state): State<ServerState>, Json(preset): Json<PresetSettings>) -> Response {
    let config: FakerConfig = preset.into();
    match state.app.set_default_config(Some(config)).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

/// Clear the default config (revert to built-in defaults)
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
async fn clear_default_config(State(state): State<ServerState>) -> Response {
    match state.app.set_default_config(None).await {
        Ok(()) => ApiSuccess::response(()),
        Err(e) => ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}
