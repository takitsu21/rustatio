pub mod config;
pub mod faker;
pub mod logger;
pub mod protocol;
pub mod torrent;
pub mod validation;

// Re-export commonly used types
pub use config::AppConfig;
pub use faker::{FakerConfig, FakerState, FakerStats, RatioFaker};
pub use protocol::{TrackerClient, TrackerError};
pub use torrent::{ClientConfig, ClientType, TorrentInfo};
pub use validation::ValidationError;
