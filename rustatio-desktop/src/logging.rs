use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

#[derive(Clone, Serialize)]
pub struct LogEvent {
    timestamp: u64,
    level: String,
    message: String,
}

pub fn emit_log(app: &AppHandle, level: &str, message: String) {
    if !rustatio_core::logger::should_emit_level(level) {
        return;
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_millis() as u64;

    let log_event = LogEvent { timestamp, level: level.to_string(), message };

    let _ = app.emit("log-event", log_event);
}

pub fn resolve_instance_label(app: &AppHandle, instance_id: u32) -> String {
    let state = app.state::<AppState>();
    let Ok(fakers) = state.fakers.try_read() else {
        return instance_id.to_string();
    };

    fakers
        .get(&instance_id)
        .map(|instance| instance.summary.name.clone())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| instance_id.to_string())
}

pub fn format_instance_message(app: &AppHandle, instance_id: u32, message: &str) -> String {
    let label = resolve_instance_label(app, instance_id);
    format!("[{label}] {message}")
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
            let msg_body = format!($($arg)*);
            let msg = $crate::logging::format_instance_message($app, $instance_id, &msg_body);
            log::info!("{}", msg);
            $crate::logging::emit_log($app, "info", msg);
        }
    };
    ($app:expr, $instance_id:expr, warn, $($arg:tt)*) => {
        {
            let msg_body = format!($($arg)*);
            let msg = $crate::logging::format_instance_message($app, $instance_id, &msg_body);
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
            let msg_body = format!($($arg)*);
            let msg = $crate::logging::format_instance_message($app, $instance_id, &msg_body);
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
