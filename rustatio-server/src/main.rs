mod api;
mod log_layer;
mod state;
mod static_files;

use axum::{routing::get, Router};
use std::net::SocketAddr;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;

use crate::log_layer::BroadcastLayer;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    // Bridge log crate to tracing FIRST (before any subscriber)
    tracing_log::LogTracer::init().expect("Failed to set logger");

    // Create shared application state (we need the log sender for the tracing layer)
    let state = AppState::new();

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

    // Get port from environment or use default
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);

    // Build CORS layer
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(|| async { "OK" }))
        // API routes
        .nest("/api", api::router())
        // Static files (web UI) - must be last as it catches all other routes
        .fallback(static_files::static_handler)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Rustatio server starting on http://{}", addr);
    tracing::info!("Web UI available at http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
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
