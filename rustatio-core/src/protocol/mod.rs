pub mod bencode;
#[cfg(not(target_arch = "wasm32"))]
pub mod peer;
pub mod tracker;

// Re-export common types
pub use bencode::BencodeError;
#[cfg(not(target_arch = "wasm32"))]
pub use peer::{peer_id_to_array, PeerHandshake, PeerProtocolError};
pub use tracker::{
    AnnounceRequest, AnnounceResponse, ScrapeResponse, TrackerClient, TrackerError, TrackerEvent,
};
