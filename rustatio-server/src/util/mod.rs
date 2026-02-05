//! Utilities module - logging and static file serving.

pub mod logging;
pub mod static_files;

// Re-export commonly used types
pub use logging::BroadcastLayer;
pub use static_files::static_handler;
