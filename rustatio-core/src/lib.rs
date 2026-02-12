pub mod config;
pub mod faker;
pub mod grid;
pub mod logger;
pub mod protocol;
pub mod torrent;
pub mod validation;

// Re-export main types explicitly to avoid ambiguous Result types
pub use config::{AppConfig, ClientSettings, ConfigError, FakerSettings, UiSettings};
pub use faker::{FakerConfig, FakerError, FakerState, FakerStats, PresetSettings, RatioFaker};
pub use grid::{GridImportSettings, GridMode, InstanceSummary};
pub use torrent::{ClientConfig, ClientInfo, ClientType, HttpVersion, TorrentError, TorrentFile, TorrentInfo};
pub use validation::*;
