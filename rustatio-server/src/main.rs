mod api;
mod services;
mod util;

use axum::{middleware, routing::get, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::{oneshot, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::{ApiDoc, ServerState};
use crate::services::{AppState, Scheduler, WatchConfig, WatchDisabledReason, WatchService};
use crate::util::BroadcastLayer;

#[tokio::main]
async fn main() {
    tracing_log::LogTracer::init().expect("Failed to set logger");

    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "/data".to_string());
    let state = AppState::new(&data_dir);

    let default_filter = "rustatio_server=info,rustatio_core=trace,log=trace,tower_http=info,hyper=info,reqwest=info";
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| default_filter.into()))
        .with(BroadcastLayer::new(state.log_sender.clone()))
        .with(tracing_subscriber::fmt::layer());

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

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

    let mut scheduler = Scheduler::new();
    scheduler.start(state.clone(), state.instances.clone());
    let scheduler = Arc::new(tokio::sync::Mutex::new(scheduler));

    let (watch_config, disabled_reason) = WatchConfig::from_env();

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

    let server_state = ServerState {
        app: state.clone(),
        watch: watch_service.clone(),
    };

    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .merge(
            SwaggerUi::new("/docs")
                .url("/api-docs/openapi.json", ApiDoc::openapi())
                .config(
                    utoipa_swagger_ui::Config::default()
                        .persist_authorization(true)
                        .try_it_out_enabled(true),
                ),
        )
        .nest("/api", api::public_router())
        .nest(
            "/api",
            api::router().layer(middleware::from_fn(api::middleware::auth_middleware)),
        )
        .fallback(util::static_handler)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(server_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Rustatio server starting on http://{}", addr);
    tracing::info!("Web UI available at http://localhost:{}", port);
    tracing::info!("API documentation at http://localhost:{}/docs", port);
    tracing::info!("Data directory: {}", data_dir);

    if api::middleware::is_auth_enabled() {
        tracing::info!("Authentication enabled (AUTH_TOKEN is set)");
    } else {
        tracing::warn!("Authentication disabled - API is open to all. Set AUTH_TOKEN to enable.");
    }

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let state_for_shutdown = state.clone();
    let watch_for_shutdown = watch_service.clone();
    let scheduler_for_shutdown = scheduler.clone();

    tokio::spawn(async move {
        shutdown_signal().await;

        tracing::info!("Stopping scheduler...");
        scheduler_for_shutdown.lock().await.shutdown().await;

        tracing::info!("Stopping watch folder service...");
        watch_for_shutdown.write().await.stop().await;

        tracing::info!("Stopping running instances...");
        state_for_shutdown.shutdown_all().await;

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
