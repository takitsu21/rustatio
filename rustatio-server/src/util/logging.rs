use crate::services::LogEvent;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

pub struct BroadcastLayer {
    sender: Arc<broadcast::Sender<LogEvent>>,
}

impl BroadcastLayer {
    pub fn new(sender: broadcast::Sender<LogEvent>) -> Self {
        Self {
            sender: Arc::new(sender),
        }
    }
}

struct MessageVisitor {
    message: String,
}

impl MessageVisitor {
    fn new() -> Self {
        Self { message: String::new() }
    }
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
            // Remove surrounding quotes if present
            if self.message.starts_with('"') && self.message.ends_with('"') {
                self.message = self.message[1..self.message.len() - 1].to_string();
            }
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}

impl<S: Subscriber> Layer<S> for BroadcastLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let target = event.metadata().target();

        // Forward logs from:
        // - "log" target (log crate events bridged via tracing-log, which includes rustatio_core)
        // - "rustatio_core" target (direct tracing events from rustatio_core)
        // Exclude other targets like tower_http, hyper, etc.
        if target != "log" && !target.starts_with("rustatio_core") {
            return;
        }

        // Extract the message
        let mut visitor = MessageVisitor::new();
        event.record(&mut visitor);

        if visitor.message.is_empty() {
            return;
        }

        // Convert level to string
        let level = match *event.metadata().level() {
            Level::ERROR => "error",
            Level::WARN => "warn",
            Level::INFO => "info",
            Level::DEBUG => "debug",
            Level::TRACE => "trace",
        };

        // Send to broadcast channel (ignore errors - no subscribers is fine)
        let _ = self.sender.send(LogEvent::new(level, visitor.message));
    }
}
