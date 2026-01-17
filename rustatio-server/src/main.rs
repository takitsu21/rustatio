mod api;
mod auth;
mod log_layer;
mod persistence;
mod state;
mod static_files;
mod watch;

use axum::{middleware, routing::get, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::{oneshot, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;

use crate::log_layer::BroadcastLayer;
use crate::state::AppState;
use crate::watch::{WatchConfig, WatchDisabledReason, WatchService};

/// Combined application state for routing
#[derive(Clone)]
pub struct ServerState {
    pub app: AppState,
    pub watch: Arc<RwLock<WatchService>>,
}

#[tokio::main]
async fn main() {
    // Bridge log crate to tracing FIRST (before any subscriber)
    tracing_log::LogTracer::init().expect("Failed to set logger");

    // Get data directory from environment or use default
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "/data".to_string());

    // Create shared application state
    let state = AppState::new(&data_dir);

    // Initialize tracing subscriber with EnvFilter and broadcast layer
    // Default: show info for server, trace for rustatio_core/log (for UI filtering)
    // The "log" target captures all log crate events bridged via tracing-log
    let default_filter = "rustatio_server=info,rustatio_core=trace,log=trace,tower_http=info,hyper=info,reqwest=info";
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| default_filter.into()))
        .with(BroadcastLayer::new(state.log_sender.clone()))
        .with(tracing_subscriber::fmt::layer());

    // Set as global default
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    // Load saved state and restore instances
    match state.load_saved_state().await {
        Ok(count) => {
            if count > 0 {
                tracing::info!("Restored {} instance(s) from saved state", count);
            }
        }
        Err(e) => {
            tracing::error!("Failed to load saved state: {}", e);
        }
    }

    // Initialize and start watch folder service
    let (watch_config, disabled_reason) = WatchConfig::from_env();

    // Log appropriate message based on watch folder status
    if let Some(reason) = &disabled_reason {
        match reason {
            WatchDisabledReason::ExplicitlyDisabled => {
                tracing::info!("Watch folder service disabled via WATCH_ENABLED=false");
            }
            WatchDisabledReason::DirectoryNotFound => {
                tracing::info!(
                    "Watch folder service disabled: directory '{}' not found. \
                    To enable, mount a volume to {} or set WATCH_ENABLED=true to auto-create it.",
                    watch_config.watch_dir.display(),
                    watch_config.watch_dir.display()
                );
            }
        }
    }

    let mut watch_service = WatchService::new(watch_config.clone(), state.clone());
    if let Err(e) = watch_service.start().await {
        tracing::error!("Failed to start watch folder service: {}", e);
    }
    let watch_service = Arc::new(RwLock::new(watch_service));

    // Create combined server state
    let server_state = ServerState {
        app: state.clone(),
        watch: watch_service.clone(),
    };

    // Get port from environment or use default
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);

    // Build CORS layer
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    // Build router
    let app = Router::new()
        // Health check (no auth required)
        .route("/health", get(|| async { "OK" }))
        // Public API routes (no auth required)
        .nest("/api", api::public_router())
        // Protected API routes (auth required when AUTH_TOKEN is set)
        .nest("/api", api::router().layer(middleware::from_fn(auth::auth_middleware)))
        // Static files (web UI) - must be last as it catches all other routes (no auth)
        .fallback(static_files::static_handler)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(server_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Rustatio server starting on http://{}", addr);
    tracing::info!("Web UI available at http://localhost:{}", port);
    tracing::info!("Data directory: {}", data_dir);

    // Log authentication status
    if auth::is_auth_enabled() {
        tracing::info!("Authentication enabled (AUTH_TOKEN is set)");
    } else {
        tracing::warn!("Authentication disabled - API is open to all. Set AUTH_TOKEN to enable.");
    }

    // Create shutdown signal channel
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let state_for_shutdown = state.clone();
    let watch_for_shutdown = watch_service.clone();

    // Spawn shutdown handler
    tokio::spawn(async move {
        shutdown_signal().await;

        // Stop watch service first
        tracing::info!("Stopping watch folder service...");
        watch_for_shutdown.write().await.stop().await;

        // Stop all background tasks
        tracing::info!("Stopping background tasks...");
        state_for_shutdown.shutdown_all().await;

        // Save state before shutting down
        tracing::info!("Saving state before shutdown...");
        if let Err(e) = state_for_shutdown.save_state().await {
            tracing::error!("Failed to save state on shutdown: {}", e);
        } else {
            tracing::info!("State saved successfully");
        }

        let _ = shutdown_tx.send(());
    });

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        })
        .await
        .unwrap();

    tracing::info!("Server shutdown complete");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, stopping server...");
}
