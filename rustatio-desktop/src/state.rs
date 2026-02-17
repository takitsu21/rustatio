use rustatio_core::{FakerConfig, RatioFaker, TorrentInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct FakerInstance {
    pub faker: Arc<RwLock<RatioFaker>>,
    pub torrent: TorrentInfo,
    pub config: FakerConfig,
    pub cumulative_uploaded: u64,
    pub cumulative_downloaded: u64,
    pub tags: Vec<String>,
    pub created_at: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: u32,
    pub torrent_name: Option<String>,
    pub is_running: bool,
    pub is_paused: bool,
}

pub struct AppState {
    pub fakers: Arc<RwLock<HashMap<u32, FakerInstance>>>,
    pub next_instance_id: Arc<RwLock<u32>>,
    pub config: Arc<RwLock<rustatio_core::AppConfig>>,
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_secs()
}

pub fn hex_info_hash(hash: &[u8; 20]) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}
