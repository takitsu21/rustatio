use rustatio_core::{FakerConfig, RatioFakerHandle, TorrentInfo, TorrentSummary};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::persistence;
use crate::persistence::{PersistedInstance, PersistedState, WatchSettings};

pub struct FakerInstance {
    pub faker: Arc<RatioFakerHandle>,
    pub torrent: Arc<TorrentInfo>,
    pub summary: Arc<TorrentSummary>,
    pub config: FakerConfig,
    pub cumulative_uploaded: u64,
    pub cumulative_downloaded: u64,
    pub tags: Vec<String>,
    pub created_at: u64,
    pub source: rustatio_watch::InstanceSource,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: u32,
    pub torrent_name: Option<String>,
    pub is_running: bool,
    pub is_paused: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub fakers: Arc<RwLock<HashMap<u32, FakerInstance>>>,
    pub next_instance_id: Arc<RwLock<u32>>,
    pub config: Arc<RwLock<rustatio_core::AppConfig>>,
    pub http_client: rustatio_core::reqwest::Client,
    pub watch: Arc<RwLock<Option<crate::watch::DesktopWatchService>>>,
    pub default_config: Arc<RwLock<Option<FakerConfig>>>,
    pub watch_settings: Arc<RwLock<Option<WatchSettings>>>,
    pub should_exit: Arc<AtomicBool>,
    pub close_prompt_open: Arc<AtomicBool>,
}

impl AppState {
    pub async fn build_persisted_state(&self) -> PersistedState {
        let fakers = self.fakers.read().await;
        let next_id = *self.next_instance_id.read().await;
        let now = persistence::now_timestamp();

        let mut instances = HashMap::new();
        for (id, instance) in fakers.iter() {
            let stats = instance.faker.stats_snapshot();
            let mut config = instance.config.clone();
            config.completion_percent = stats.torrent_completion;
            instances.insert(
                *id,
                PersistedInstance {
                    id: *id,
                    torrent: (*instance.summary).clone(),
                    config,
                    cumulative_uploaded: stats.uploaded,
                    cumulative_downloaded: stats.downloaded,
                    state: stats.state,
                    created_at: instance.created_at,
                    updated_at: now,
                    tags: instance.tags.clone(),
                    from_watch_folder: matches!(
                        instance.source,
                        rustatio_watch::InstanceSource::WatchFolder
                    ),
                },
            );
        }

        let default_config = self.default_config.read().await.clone();
        let watch_settings = self.watch_settings.read().await.clone();

        PersistedState {
            instances,
            next_instance_id: next_id,
            default_config,
            watch_settings,
            version: 1,
        }
    }

    pub async fn save_state(&self) -> Result<(), String> {
        let persisted = self.build_persisted_state().await;
        persistence::save_state(&persisted)
    }
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_secs()
}

pub fn hex_info_hash(hash: &[u8; 20]) -> String {
    hash.iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}
