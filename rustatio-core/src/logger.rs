// Platform-agnostic logger module
// Native (desktop) uses Tauri Emitter, WASM uses web_sys console

#[cfg(all(not(target_arch = "wasm32"), feature = "native"))]
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
        log::info!("{}", message);
        emit_log("info", message);
    }

    /// Log at warn level (both to console and UI)
    pub fn warn(message: String) {
        log::warn!("{}", message);
        emit_log("warn", message);
    }

    /// Log at error level (both to console and UI)
    pub fn error(message: String) {
        log::error!("{}", message);
        emit_log("error", message);
    }

    /// Log at debug level (both to console and UI)
    pub fn debug(message: String) {
        log::debug!("{}", message);
        emit_log("debug", message);
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
        emit_log("info", &message);
    }

    /// Log at warn level to browser console and UI
    pub fn warn(message: String) {
        emit_log("warn", &message);
    }

    /// Log at error level to browser console and UI
    pub fn error(message: String) {
        emit_log("error", &message);
    }

    /// Log at debug level to browser console and UI
    pub fn debug(message: String) {
        emit_log("debug", &message);
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
