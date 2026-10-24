//! Server-Sent Events (SSE) streaming endpoints.

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Router,
};
use futures::stream::Stream;
use rustatio_core::InstanceSummary;
use std::convert::Infallible;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::api::ServerState;
use crate::services::{EventBroadcaster, InstanceEvent, LogEvent};

const fn instance_event_name(instance_event: &InstanceEvent) -> &'static str {
    // Keep the existing `instance` event name for lifecycle events (created/deleted)
    // so existing custom scripts are unaffected; summaries get their own event name.
    match instance_event {
        InstanceEvent::Summaries { .. } => "summaries",
        _ => "instance",
    }
}

fn to_sse_event(instance_event: &InstanceEvent) -> Event {
    Event::default()
        .event(instance_event_name(instance_event))
        .json_data(instance_event)
        .unwrap_or_else(|_| Event::default())
}

fn to_log_event(log_event: &LogEvent) -> Event {
    Event::default().event("log").json_data(log_event).unwrap_or_else(|_| Event::default())
}

fn combined_stream(
    log_rx: broadcast::Receiver<LogEvent>,
    instance_rx: broadcast::Receiver<InstanceEvent>,
    summaries: Vec<InstanceSummary>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    // Send a full snapshot on connect so clients start consistent before live updates arrive.
    let initial =
        futures::stream::iter([Ok::<Event, Infallible>(to_sse_event(&InstanceEvent::Summaries {
            instances: summaries,
        }))]);

    let instances = BroadcastStream::new(instance_rx)
        .filter_map(|result| result.ok().map(|event| Ok(to_sse_event(&event))));
    let logs = BroadcastStream::new(log_rx)
        .filter_map(|result| result.ok().map(|event| Ok(to_log_event(&event))));

    initial.chain(instances.merge(logs))
}

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
pub async fn logs_sse(
    State(state): State<ServerState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.app.subscribe_logs();

    let stream = BroadcastStream::new(rx)
        .filter_map(|result| result.ok().map(|log_event| Ok(to_log_event(&log_event))));

    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[utoipa::path(
    get,
    path = "/events",
    tag = "events",
    summary = "Stream instance events via SSE",
    description = "Server-Sent Events stream for real-time instance updates. Lifecycle events use the 'instance' event name with InstanceEvent data (created/deleted). Stats batches use the 'summaries' event name with a Summaries payload; existing 'instance' listeners are unaffected.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "SSE stream established", content_type = "text/event-stream"),
        (status = 401, description = "Unauthorized", body = crate::api::common::ApiError)
    )
)]
pub async fn instances_sse(
    State(state): State<ServerState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.app.subscribe_instance_events();

    // Send a full snapshot on connect so clients start consistent before live updates arrive.
    let summaries = state.app.list_instance_summaries().await;
    let initial =
        futures::stream::iter([Ok::<Event, Infallible>(to_sse_event(&InstanceEvent::Summaries {
            instances: summaries,
        }))]);

    let live = BroadcastStream::new(rx)
        .filter_map(|result| result.ok().map(|event| Ok(to_sse_event(&event))));

    Sse::new(initial.chain(live)).keep_alive(KeepAlive::default())
}

#[utoipa::path(
    get,
    path = "/stream",
    tag = "events",
    summary = "Stream logs and instance events via SSE",
    description = "Multiplexed Server-Sent Events stream for clients that need both logs and instance updates over one connection. Log messages use the 'log' event name. Lifecycle events use the 'instance' event name with InstanceEvent data (created/deleted). Stats batches use the 'summaries' event name with a Summaries payload, sent once on connect and then on every update. The separate '/logs' and '/events' endpoints remain available.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "SSE stream established", content_type = "text/event-stream"),
        (status = 401, description = "Unauthorized", body = crate::api::common::ApiError)
    )
)]
pub async fn stream_sse(
    State(state): State<ServerState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let log_rx = state.app.subscribe_logs();
    let instance_rx = state.app.subscribe_instance_events();
    let summaries = state.app.list_instance_summaries().await;

    Sse::new(combined_stream(log_rx, instance_rx, summaries)).keep_alive(KeepAlive::default())
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/logs", get(logs_sse))
        .route("/events", get(instances_sse))
        .route("/stream", get(stream_sse))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::HttpBody;
    use axum::response::IntoResponse;
    use std::pin::Pin;

    async fn event_text(event: Event) -> String {
        let mut body = Sse::new(futures::stream::iter([Ok::<Event, Infallible>(event)]))
            .into_response()
            .into_body();
        let frame = futures::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx)).await;

        let Some(Ok(frame)) = frame else {
            return String::new();
        };
        let Ok(data) = frame.into_data() else {
            return String::new();
        };

        String::from_utf8_lossy(&data).into_owned()
    }

    async fn next_text<S>(stream: &mut S) -> String
    where
        S: Stream<Item = Result<Event, Infallible>> + Unpin,
    {
        match stream.next().await {
            Some(Ok(event)) => event_text(event).await,
            _ => String::new(),
        }
    }

    #[test]
    fn test_instance_event_names() {
        let created = InstanceEvent::Created {
            id: "a".to_string(),
            torrent_name: "sample".to_string(),
            info_hash: "hash".to_string(),
            auto_started: false,
        };
        let deleted = InstanceEvent::Deleted { id: "a".to_string() };
        let summaries = InstanceEvent::Summaries { instances: Vec::new() };

        assert_eq!(instance_event_name(&created), "instance");
        assert_eq!(instance_event_name(&deleted), "instance");
        assert_eq!(instance_event_name(&summaries), "summaries");
    }

    #[tokio::test]
    async fn test_combined_stream_keeps_names_and_snapshot() {
        let (log_tx, log_rx) = broadcast::channel(4);
        let (instance_tx, instance_rx) = broadcast::channel(4);
        let mut stream = Box::pin(combined_stream(log_rx, instance_rx, Vec::new()));

        let snapshot = next_text(&mut stream).await;
        assert!(snapshot.contains("event: summaries"));
        assert!(snapshot.contains("\"type\":\"summaries\""));

        let _ = log_tx.send(LogEvent {
            timestamp: 1,
            level: "info".to_string(),
            message: "hello".to_string(),
        });
        let log = next_text(&mut stream).await;
        assert!(log.contains("event: log"));
        assert!(log.contains("\"message\":\"hello\""));

        let _ = instance_tx.send(InstanceEvent::Deleted { id: "a".to_string() });
        let instance = next_text(&mut stream).await;
        assert!(instance.contains("event: instance"));
        assert!(instance.contains("\"type\":\"deleted\""));
    }
}
