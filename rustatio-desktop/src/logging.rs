use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
pub struct LogEvent {
    timestamp: u64,
    level: String,
    message: String,
}

pub fn emit_log(app: &AppHandle, level: &str, message: String) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_millis() as u64;

    let log_event = LogEvent {
        timestamp,
        level: level.to_string(),
        message,
    };

    let _ = app.emit("log-event", log_event);
}

macro_rules! log_and_emit {
    ($app:expr, info, $($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            log::info!("{}", msg);
            $crate::logging::emit_log($app, "info", msg);
        }
    };
    ($app:expr, warn, $($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            log::warn!("{}", msg);
            $crate::logging::emit_log($app, "warn", msg);
        }
    };
    ($app:expr, $instance_id:expr, info, $($arg:tt)*) => {
        {
            let msg = format!("[Instance {}] {}", $instance_id, format!($($arg)*));
            log::info!("{}", msg);
            $crate::logging::emit_log($app, "info", msg);
        }
    };
    ($app:expr, $instance_id:expr, warn, $($arg:tt)*) => {
        {
            let msg = format!("[Instance {}] {}", $instance_id, format!($($arg)*));
            log::warn!("{}", msg);
            $crate::logging::emit_log($app, "warn", msg);
        }
    };
    ($app:expr, error, $($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            log::error!("{}", msg);
            $crate::logging::emit_log($app, "error", msg);
        }
    };
    ($app:expr, $instance_id:expr, error, $($arg:tt)*) => {
        {
            let msg = format!("[Instance {}] {}", $instance_id, format!($($arg)*));
            log::error!("{}", msg);
            $crate::logging::emit_log($app, "error", msg);
        }
    };
    ($app:expr, debug, $($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            log::debug!("{}", msg);
            $crate::logging::emit_log($app, "debug", msg);
        }
    };
}

pub(crate) use log_and_emit;
