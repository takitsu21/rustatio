use rustatio_core::InstanceSummary;
use serde::Serialize;
use tokio::sync::broadcast;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct LogEvent {
    pub timestamp: u64,
    pub level: String,
    pub message: String,
}

impl LogEvent {
    pub fn new(level: &str, message: String) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self { timestamp, level: level.to_string(), message }
    }
}

#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstanceEvent {
    Created {
        id: String,
        torrent_name: String,
        info_hash: String,
        auto_started: bool,
    },
    Deleted {
        id: String,
    },
    Summaries {
        #[schema(value_type = Vec<Object>)]
        instances: Vec<InstanceSummary>,
    },
}

pub trait EventBroadcaster {
    fn subscribe_logs(&self) -> broadcast::Receiver<LogEvent>;
    fn subscribe_instance_events(&self) -> broadcast::Receiver<InstanceEvent>;
    fn emit_instance_event(&self, event: InstanceEvent);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_events_keep_their_shape() {
        let event = InstanceEvent::Deleted { id: "abc".to_string() };
        let value = serde_json::to_value(&event).unwrap_or(serde_json::Value::Null);
        assert_eq!(value["type"], "deleted");
        assert_eq!(value["id"], "abc");
    }

    #[test]
    fn test_summaries_event_serializes_with_tag() {
        let event = InstanceEvent::Summaries { instances: Vec::new() };
        let value = serde_json::to_value(&event).unwrap_or(serde_json::Value::Null);
        assert_eq!(value["type"], "summaries");
        assert!(value["instances"].is_array());
    }
}
