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
        Self {
            timestamp,
            level: level.to_string(),
            message,
        }
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
}

pub trait EventBroadcaster {
    fn subscribe_logs(&self) -> broadcast::Receiver<LogEvent>;
    fn subscribe_instance_events(&self) -> broadcast::Receiver<InstanceEvent>;
    fn emit_instance_event(&self, event: InstanceEvent);
}
