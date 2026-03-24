use rustatio_core::{
    FakerConfig, FakerState, PeerListenerService, PeerListenerStatus, RatioFakerHandle,
    TorrentInfo, TorrentSummary,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::persistence;
use crate::persistence::{PersistedInstance, PersistedState, WatchSettings};

type PeerListenerHandle = Arc<Mutex<PeerListenerService>>;

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
    pub peer_listener: Arc<RwLock<Option<PeerListenerHandle>>>,
    pub peer_listener_status: Arc<RwLock<PeerListenerStatus>>,
}

impl AppState {
    pub async fn attach_peer_listener(&self, listener: PeerListenerHandle) {
        *self.peer_listener.write().await = Some(listener);
        self.refresh_peer_listener_port().await;
    }

    pub async fn set_peer_listener_status(&self, status: PeerListenerStatus) {
        *self.peer_listener_status.write().await = status;
    }

    pub async fn peer_listener_status(&self) -> PeerListenerStatus {
        self.peer_listener_status.read().await.clone()
    }

    async fn desired_peer_port_from_instances(&self) -> Result<Option<u16>, String> {
        let fakers = self.fakers.read().await;
        let mut ports = BTreeSet::new();

        for instance in fakers.values() {
            let state = instance.faker.stats_snapshot().state;
            if matches!(state, FakerState::Starting | FakerState::Running | FakerState::Paused) {
                ports.insert(instance.config.port);
            }
        }

        match ports.len() {
            0 => Ok(None),
            1 => Ok(ports.iter().next().copied()),
            _ => Err(format!(
                "multiple active peer ports configured: {}",
                ports.iter().map(u16::to_string).collect::<Vec<_>>().join(", ")
            )),
        }
    }

    pub async fn refresh_peer_listener_port(&self) {
        let listener = self.peer_listener.read().await.clone();
        let Some(listener) = listener else {
            return;
        };

        match self.desired_peer_port_from_instances().await {
            Ok(port) => {
                listener.lock().await.set_desired_port(port);
            }
            Err(err) => {
                log::warn!("Peer listener disabled: {err}");
                listener.lock().await.set_desired_port(None);
                let current = self.peer_listener_status().await;
                self.set_peer_listener_status(PeerListenerStatus {
                    enabled: true,
                    desired_port: None,
                    bound_port: None,
                    active_torrents: current.active_torrents,
                    last_error: Some(err),
                })
                .await;
            }
        }
    }

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
