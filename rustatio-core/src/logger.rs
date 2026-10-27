// Platform-agnostic logger module
// Desktop uses Tauri Emitter, CLI uses standard logging, WASM uses web_sys console

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// Thread-local storage for instance context (string-based for server compatibility)
thread_local! {
    static THREAD_CONTEXT: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn swap_context(slot: &mut Option<String>) {
    let _ = THREAD_CONTEXT.try_with(|ctx| std::mem::swap(&mut *ctx.borrow_mut(), slot));
}

/// Échange le contexte du thread avec `slot`, et le rétablit à la sortie (même en cas de panic).
struct SwapGuard<'a>(&'a mut Option<String>);

impl<'a> SwapGuard<'a> {
    fn new(slot: &'a mut Option<String>) -> Self {
        swap_context(slot);
        Self(slot)
    }
}

impl Drop for SwapGuard<'_> {
    fn drop(&mut self) {
        swap_context(self.0);
    }
}

/// Future qui pose le contexte d'instance à chaque `poll`, quel que soit le thread.
#[must_use = "futures do nothing unless awaited"]
pub struct WithInstanceContext<F> {
    slot: Option<String>,
    fut: Pin<Box<F>>,
}

impl<F: Future> Future for WithInstanceContext<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let _guard = SwapGuard::new(&mut this.slot);
        this.fut.as_mut().poll(cx)
    }
}

/// Exécute `fut` avec ce contexte d'instance. Le contexte suit la tâche, même si elle
/// change de thread après un `.await`
#[allow(clippy::needless_pass_by_value)]
pub fn with_instance_context<F: Future>(
    instance_id: impl ToString,
    fut: F,
) -> WithInstanceContext<F> {
    WithInstanceContext { slot: Some(instance_id.to_string()), fut: Box::pin(fut) }
}

/// Version synchrone de [`with_instance_context`].
#[allow(clippy::needless_pass_by_value)]
pub fn with_instance_context_sync<R>(instance_id: impl ToString, f: impl FnOnce() -> R) -> R {
    let mut slot = Some(instance_id.to_string());
    let _guard = SwapGuard::new(&mut slot);
    f()
}

/// Set the instance context for the current thread (string version for server/wasm).
/// Code synchrone uniquement : ne pas utiliser avant un `.await`, préférer `with_instance_context`.
pub fn set_instance_context_str(instance_id: Option<&str>) {
    THREAD_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = instance_id.map(std::string::ToString::to_string);
    });
}

/// Set the instance context for the current thread (u32 version for desktop/cli compatibility).
/// Code synchrone uniquement : ne pas utiliser avant un `.await`, préférer `with_instance_context`.
pub fn set_instance_context(instance_id: Option<u32>) {
    THREAD_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = instance_id.map(|id| id.to_string());
    });
}

/// Get the current instance context
fn get_instance_prefix() -> String {
    THREAD_CONTEXT
        .with(|ctx| ctx.borrow().as_ref().map_or_else(String::new, |label| format!("[{label}] ")))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub mod native {
    use serde::Serialize;
    use std::sync::atomic::{AtomicU8, Ordering};
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

    // Maximum log level to emit via IPC (0=error, 1=warn, 2=info, 3=debug, 4=trace)
    // Defaults to info (2) — matches the frontend default
    static MAX_EMIT_LEVEL: AtomicU8 = AtomicU8::new(2);

    // Wrapper to make AppHandle Send + Sync
    struct AppHandleWrapper {
        handle: tauri::AppHandle,
    }

    // Safety: tauri::AppHandle is internally Arc-based and thread-safe,
    // but doesn't implement Send/Sync due to platform abstractions.
    #[allow(unsafe_code)]
    unsafe impl Send for AppHandleWrapper {}
    #[allow(unsafe_code)]
    unsafe impl Sync for AppHandleWrapper {}

    fn level_to_u8(level: &str) -> u8 {
        match level {
            "error" => 0,
            "warn" => 1,
            "debug" => 3,
            "trace" => 4,
            _ => 2,
        }
    }

    /// Set the maximum log level for IPC emission (called from frontend via Tauri command)
    pub fn set_max_emit_level(level: &str) {
        MAX_EMIT_LEVEL.store(level_to_u8(level), Ordering::Relaxed);
    }

    /// Check if a log level should be emitted via IPC
    pub fn should_emit_level(level: &str) -> bool {
        level_to_u8(level) <= MAX_EMIT_LEVEL.load(Ordering::Relaxed)
    }

    /// Initialize the logger with the app handle
    pub fn init_logger(handle: tauri::AppHandle) {
        let _ = APP_HANDLE.set(AppHandleWrapper { handle });
    }

    fn emit_log(level: &str, message: String) {
        if !should_emit_level(level) {
            return;
        }
        if let Some(wrapper) = APP_HANDLE.get() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_millis() as u64;

            let log_event = LogEvent { timestamp, level: level.to_string(), message };

            let _ = wrapper.handle.emit("log-event", log_event);
        }
    }

    pub fn info(message: &str) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::info!("{prefixed}");
        emit_log("info", prefixed);
    }

    pub fn warn(message: &str) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::warn!("{prefixed}");
        emit_log("warn", prefixed);
    }

    pub fn error(message: &str) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::error!("{prefixed}");
        emit_log("error", prefixed);
    }

    pub fn debug(message: &str) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::debug!("{prefixed}");
        emit_log("debug", prefixed);
    }

    pub fn trace(message: &str) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::trace!("{prefixed}");
        emit_log("trace", prefixed);
    }
}

// CLI logger - native without desktop (no Tauri)
#[cfg(all(not(target_arch = "wasm32"), feature = "native", not(feature = "desktop")))]
pub mod native {
    /// No-op for CLI (no app handle needed)
    pub const fn init_logger() {
        // No initialization needed for CLI
    }

    /// Log at info level
    pub fn info(message: &str) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::info!("{prefixed}");
    }

    /// Log at warn level
    pub fn warn(message: &str) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::warn!("{prefixed}");
    }

    /// Log at error level
    pub fn error(message: &str) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::error!("{prefixed}");
    }

    /// Log at debug level
    pub fn debug(message: &str) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::debug!("{prefixed}");
    }

    /// Log at trace level
    pub fn trace(message: &str) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        log::trace!("{prefixed}");
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
            "debug" | "trace" => console_debug(message),
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
    pub fn info(message: &str) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        emit_log("info", &prefixed);
    }

    /// Log at warn level to browser console and UI
    pub fn warn(message: &str) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        emit_log("warn", &prefixed);
    }

    /// Log at error level to browser console and UI
    pub fn error(message: &str) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        emit_log("error", &prefixed);
    }

    /// Log at debug level to browser console and UI
    pub fn debug(message: &str) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        emit_log("debug", &prefixed);
    }

    /// Log at trace level to browser console and UI
    pub fn trace(message: &str) {
        let prefixed = format!("{}{}", super::get_instance_prefix(), message);
        emit_log("trace", &prefixed);
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
        $crate::logger::info(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logger::warn(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logger::error(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logger::debug(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        $crate::logger::trace(&format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_context_str_prefix() {
        set_instance_context_str(Some("abc"));
        assert_eq!(get_instance_prefix(), "[abc] ");
        set_instance_context_str(None);
        assert_eq!(get_instance_prefix(), "");
    }

    #[test]
    fn test_instance_context_prefix() {
        set_instance_context(Some(7));
        assert_eq!(get_instance_prefix(), "[7] ");
        set_instance_context(None);
        assert_eq!(get_instance_prefix(), "");
    }

    #[test]
    fn test_context_follows_future_when_polls_interleave() {
        use std::task::Waker;

        struct YieldOnce(bool);
        impl Future for YieldOnce {
            type Output = ();
            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
                if self.0 {
                    Poll::Ready(())
                } else {
                    self.0 = true;
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }

        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut task_a = Box::pin(with_instance_context("task_a", async {
            YieldOnce(false).await;
            get_instance_prefix()
        }));
        let mut task_b = Box::pin(with_instance_context("task_b", async {
            YieldOnce(false).await;
            get_instance_prefix()
        }));

        assert!(task_a.as_mut().poll(&mut cx).is_pending());
        assert!(task_b.as_mut().poll(&mut cx).is_pending());
        assert_eq!(task_a.as_mut().poll(&mut cx), Poll::Ready("[task_a] ".to_string()));
        assert_eq!(task_b.as_mut().poll(&mut cx), Poll::Ready("[task_b] ".to_string()));
        assert_eq!(get_instance_prefix(), "");
    }
}
