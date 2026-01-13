// Platform-agnostic logger module
// Desktop uses Tauri Emitter, CLI uses standard logging, WASM uses web_sys console

use std::cell::RefCell;

// Thread-local storage for instance context
thread_local! {
    static INSTANCE_CONTEXT: RefCell<Option<u32>> = const { RefCell::new(None) };
}

/// Set the instance context for the current thread
pub fn set_instance_context(instance_id: Option<u32>) {
    INSTANCE_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = instance_id;
    });
}

/// Get the current instance context
fn get_instance_prefix() -> String {
    INSTANCE_CONTEXT.with(|ctx| {
        if let Some(id) = *ctx.borrow() {
            format!("[Instance {}] ", id)
        } else {
            String::new()
        }
    })
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub mod native {
    use serde::Serialize;
    use std::sync::OnceLock;
    use tauri::Emitter;

    // Log event payload
    #[derive(Clone, Serialize)]
    struct LogEvent {
        timestamp: u64,
        level: String,
        message: String,
    }

    // Global app handle storage
    static APP_HANDLE: OnceLock<AppHandleWrapper> = OnceLock::new();

    // Wrapper to make AppHandle Send + Sync
    struct AppHandleWrapper {
        handle: tauri::AppHandle,
    }

    unsafe impl Send for AppHandleWrapper {}
    unsafe impl Sync for AppHandleWrapper {}

    /// Initialize the logger with the app handle
    pub fn init_logger(handle: tauri::AppHandle) {
        let _ = APP_HANDLE.set(AppHandleWrapper { handle });
    }

    /// Emit a log to the UI (if app handle is available)
    fn emit_log(level: &str, message: String) {
        if let Some(wrapper) = APP_HANDLE.get() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_millis() as u64;

            let log_event = LogEvent {
                timestamp,
                level: level.to_string(),
                message: message.clone(),
            };

            let _ = wrapper.handle.emit("log-event", log_event);
        }
    }

    /// Log at info level (both to console and UI)
    pub fn info(message: String) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::info!("{}", prefixed);
        emit_log("info", prefixed);
    }

    /// Log at warn level (both to console and UI)
    pub fn warn(message: String) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::warn!("{}", prefixed);
        emit_log("warn", prefixed);
    }

    /// Log at error level (both to console and UI)
    pub fn error(message: String) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::error!("{}", prefixed);
        emit_log("error", prefixed);
    }

    /// Log at debug level (both to console and UI)
    pub fn debug(message: String) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::debug!("{}", prefixed);
        emit_log("debug", prefixed);
    }
}

// CLI logger - native without desktop (no Tauri)
#[cfg(all(not(target_arch = "wasm32"), feature = "native", not(feature = "desktop")))]
pub mod native {
    /// No-op for CLI (no app handle needed)
    pub fn init_logger() {
        // No initialization needed for CLI
    }

    /// Log at info level
    pub fn info(message: String) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::info!("{}", prefixed);
    }

    /// Log at warn level
    pub fn warn(message: String) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::warn!("{}", prefixed);
    }

    /// Log at error level
    pub fn error(message: String) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::error!("{}", prefixed);
    }

    /// Log at debug level
    pub fn debug(message: String) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::debug!("{}", prefixed);
    }
}

#[cfg(target_arch = "wasm32")]
pub mod wasm {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console, js_name = log)]
        fn console_log(s: &str);
        #[wasm_bindgen(js_namespace = console, js_name = warn)]
        fn console_warn(s: &str);
        #[wasm_bindgen(js_namespace = console, js_name = error)]
        fn console_error(s: &str);
        #[wasm_bindgen(js_namespace = console, js_name = debug)]
        fn console_debug(s: &str);
    }

    // Store log callback - will be set from JavaScript
    thread_local! {
        static LOG_CALLBACK: std::cell::RefCell<Option<js_sys::Function>> = std::cell::RefCell::new(None);
    }

    /// Set the JavaScript callback for log events (called from JS during init)
    #[wasm_bindgen]
    pub fn set_log_callback(callback: js_sys::Function) {
        LOG_CALLBACK.with(|cb| {
            *cb.borrow_mut() = Some(callback);
        });
    }

    /// Internal helper to emit log to both console and callback
    fn emit_log(level: &str, message: &str) {
        // Log to console
        match level {
            "error" => console_error(message),
            "warn" => console_warn(message),
            "debug" => console_debug(message),
            _ => console_log(message),
        }

        // Call JavaScript callback if set
        LOG_CALLBACK.with(|cb| {
            if let Some(callback) = cb.borrow().as_ref() {
                let this = JsValue::NULL;
                let level_js = JsValue::from_str(level);
                let message_js = JsValue::from_str(message);
                let _ = callback.call2(&this, &level_js, &message_js);
            }
        });
    }

    /// Log at info level to browser console and UI
    pub fn info(message: String) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        emit_log("info", &prefixed);
    }

    /// Log at warn level to browser console and UI
    pub fn warn(message: String) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        emit_log("warn", &prefixed);
    }

    /// Log at error level to browser console and UI
    pub fn error(message: String) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        emit_log("error", &prefixed);
    }

    /// Log at debug level to browser console and UI
    pub fn debug(message: String) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        emit_log("debug", &prefixed);
    }

    /// No-op for WASM (no app handle needed)
    pub fn init_logger() {
        // No initialization needed for WASM
    }
}

// Re-export based on platform
#[cfg(all(not(target_arch = "wasm32"), feature = "native"))]
pub use native::*;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;

// Macros work for both platforms
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logger::info(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logger::warn(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logger::error(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logger::debug(format!($($arg)*))
    };
}
