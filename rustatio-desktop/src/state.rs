use rustatio_core::{FakerConfig, RatioFaker, TorrentInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::persistence;

pub struct FakerInstance {
    pub faker: RatioFaker,
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

impl AppState {
    pub async fn save_state(&self) {
        let fakers = self.fakers.read().await;
        let next_id = *self.next_instance_id.read().await;
        let now = persistence::now_timestamp();

        let mut instances = HashMap::new();
        for (id, instance) in fakers.iter() {
            let stats = instance.faker.get_stats().await;
            let mut config = instance.config.clone();
            config.completion_percent = stats.torrent_completion;
            instances.insert(
                *id,
                persistence::PersistedInstance {
                    id: *id,
                    torrent: instance.torrent.clone(),
                    config,
                    cumulative_uploaded: stats.uploaded,
                    cumulative_downloaded: stats.downloaded,
                    state: stats.state,
                    created_at: instance.created_at,
                    updated_at: now,
                    tags: instance.tags.clone(),
                },
            );
        }

        let state = persistence::PersistedState {
            instances,
            next_instance_id: next_id,
            version: 1,
        };

        if let Err(e) = persistence::save_state(&state) {
            log::error!("Failed to save state: {}", e);
        }
    }
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
