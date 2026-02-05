//! Server-Sent Events (SSE) streaming endpoints.

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Router,
};
use futures::stream::Stream;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::api::ServerState;
use crate::services::EventBroadcaster;

#[utoipa::path(
    get,
    path = "/logs",
    tag = "events",
    summary = "Stream logs via SSE",
    description = "Server-Sent Events stream for real-time log messages. Events are of type 'log' with LogEvent data.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "SSE stream established", content_type = "text/event-stream"),
        (status = 401, description = "Unauthorized", body = crate::api::common::ApiError)
    )
)]
pub async fn logs_sse(State(state): State<ServerState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
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

#[utoipa::path(
    get,
    path = "/events",
    tag = "events",
    summary = "Stream instance events via SSE",
    description = "Server-Sent Events stream for real-time instance updates. Events are of type 'instance' with InstanceEvent data (created/deleted).",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "SSE stream established", content_type = "text/event-stream"),
        (status = 401, description = "Unauthorized", body = crate::api::common::ApiError)
    )
)]
pub async fn instances_sse(State(state): State<ServerState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
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

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/logs", get(logs_sse))
        .route("/events", get(instances_sse))
}
