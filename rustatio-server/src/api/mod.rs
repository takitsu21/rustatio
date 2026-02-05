//! HTTP API module - router assembly, OpenAPI documentation, and shared types.

pub mod common;
pub mod middleware;
pub mod routes;

use std::sync::Arc;
use tokio::sync::RwLock;

use axum::Router;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::services::state::AppState;
use crate::services::watch::WatchService;

#[derive(Clone)]
pub struct ServerState {
    pub app: AppState,
    pub watch: Arc<RwLock<WatchService>>,
}

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
        // Auth
        routes::auth::auth_status,
        routes::auth::verify_auth,
        // Instances
        routes::instances::list_instances,
        routes::instances::create_instance,
        routes::instances::delete_instance,
        routes::instances::load_instance_torrent,
        routes::instances::update_instance_config,
        // Torrents
        routes::torrents::load_torrent,
        // Faker
        routes::faker::start_faker,
        routes::faker::stop_faker,
        routes::faker::pause_faker,
        routes::faker::resume_faker,
        routes::faker::update_faker,
        routes::faker::update_stats_only,
        routes::faker::get_stats,
        // Clients
        routes::clients::get_client_types,
        routes::clients::get_client_infos,
        // Network
        routes::network::get_network_status,
        // Watch
        routes::watch::get_watch_status,
        routes::watch::list_watch_files,
        routes::watch::delete_watch_file,
        routes::watch::reload_watch_file,
        routes::watch::reload_all_watch_files,
        // Config
        routes::config::get_default_config,
        routes::config::set_default_config,
        routes::config::clear_default_config,
        // Events
        routes::events::logs_sse,
        routes::events::instances_sse,
    ),
    components(
        schemas(
            common::ApiError,
            common::ApiSuccess<common::EmptyData>,
            common::EmptyData,
            routes::auth::AuthStatusResponse,
            routes::instances::CreateInstanceResponse,
            routes::instances::DeleteInstanceQuery,
            routes::torrents::LoadTorrentResponse,
            routes::faker::StartFakerRequest,
            routes::network::NetworkStatus,
            routes::watch::ReloadAllResponse,
            routes::clients::ClientInfoResponse,
            crate::services::InstanceInfo,
            crate::services::LogEvent,
            crate::services::InstanceEvent,
            crate::services::persistence::InstanceSource,
            crate::services::watch::WatchStatus,
            crate::services::watch::WatchedFile,
            crate::services::watch::WatchedFileStatus,
        )
    ),
    modifiers(&SecurityAddon),
    servers(
        (url = "/api", description = "API base path")
    )
)]
pub struct ApiDoc;

pub fn router() -> Router<ServerState> {
    Router::new()
        .merge(routes::auth::router())
        .merge(routes::instances::router())
        .merge(routes::torrents::router())
        .merge(routes::faker::router())
        .merge(routes::clients::router())
        .merge(routes::network::router())
        .merge(routes::watch::router())
        .merge(routes::config::router())
        .merge(routes::events::router())
}

pub fn public_router() -> Router<ServerState> {
    routes::auth::public_router()
}
