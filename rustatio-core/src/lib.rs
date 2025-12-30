pub mod config;
pub mod faker;
pub mod logger;
pub mod protocol;
pub mod torrent;
pub mod validation;

// Re-export main types explicitly to avoid ambiguous Result types
pub use config::{AppConfig, ClientSettings, ConfigError, FakerSettings, InstanceConfig, UiSettings};
pub use faker::{FakerConfig, FakerError, FakerState, FakerStats, RatioFaker};
pub use torrent::{ClientConfig, ClientType, HttpVersion, TorrentError, TorrentFile, TorrentInfo};
pub use validation::*;
