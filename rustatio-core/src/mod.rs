pub mod config;
pub mod faker;
pub mod logger;
#[cfg(not(target_arch = "wasm32"))]
pub mod peer_listener;
pub mod protocol;
pub mod torrent;
pub mod validation;

// Re-export commonly used types
pub use config::AppConfig;
#[cfg(not(target_arch = "wasm32"))]
pub use faker::RatioFakerHandle;
pub use faker::{FakerConfig, FakerState, FakerStats, RatioFaker};
#[cfg(not(target_arch = "wasm32"))]
pub use peer_listener::{PeerCatalog, PeerListenerService, PeerListenerStatus, PeerLookup};
pub use protocol::{TrackerClient, TrackerError};
pub use torrent::{ClientConfig, ClientType, TorrentInfo, TorrentSummary};
pub use validation::ValidationError;
