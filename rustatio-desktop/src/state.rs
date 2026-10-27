use rustatio_core::{
    primary_tracker_host, FakerConfig, FakerState, FakerStats, InstanceSummary,
    PeerListenerService, PeerListenerStatus, ProxySettings, RatioFakerHandle, TorrentInfo,
    TorrentSummary,
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

pub fn build_summary(id: u32, instance: &FakerInstance, stats: &FakerStats) -> InstanceSummary {
    let state = match stats.state {
        FakerState::Paused => "paused",
        _ if stats.is_idling => "idle",
        FakerState::Idle => "idle",
        FakerState::Starting => "starting",
        FakerState::Running => "running",
        FakerState::Stopping => "stopping",
        FakerState::Stopped => "stopped",
    };

    InstanceSummary {
        id: id.to_string(),
        name: instance.summary.name.clone(),
        info_hash: hex_info_hash(&instance.summary.info_hash),
        primary_tracker_host: primary_tracker_host(&instance.summary.announce),
        state: state.to_string(),
        is_tracker_invalid: stats.tracker_error.is_some(),
        tracker_error: stats.tracker_error.clone(),
        tracker_retry_attempt: stats.tracker_retry_attempt,
        tracker_retry_at_ms: stats.tracker_retry_at_ms,
        tags: instance.tags.clone(),
        total_size: instance.summary.total_size,
        uploaded: stats.uploaded,
        downloaded: stats.downloaded,
        ratio: stats.ratio,
        current_upload_rate: stats.current_upload_rate,
        current_download_rate: stats.current_download_rate,
        seeders: stats.seeders,
        leechers: stats.leechers,
        left: stats.left,
        torrent_completion: stats.torrent_completion,
        overall_progress: stats.overall_progress(&instance.config),
        source: match instance.source {
            rustatio_watch::InstanceSource::Manual => "manual",
            rustatio_watch::InstanceSource::WatchFolder => "watch_folder",
        }
        .to_string(),
        created_at: instance.created_at,
        is_idling: stats.is_idling,
        idling_reason: stats.idling_reason.clone(),
        stop_condition_met: stats.stop_condition_met,
        post_stop_action: stats.post_stop_action,
        effective_stop_at_ratio: stats.effective_stop_at_ratio,
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: String,
    pub torrent: TorrentSummary,
    pub config: FakerConfig,
    pub stats: FakerStats,
    pub created_at: u64,
    pub source: String,
    pub tags: Vec<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub fakers: Arc<RwLock<HashMap<u32, FakerInstance>>>,
    pub next_instance_id: Arc<RwLock<u32>>,
    pub config: Arc<RwLock<rustatio_core::AppConfig>>,
    pub http_client: Arc<RwLock<rustatio_core::reqwest::Client>>,
    pub proxy_settings: Arc<RwLock<Option<ProxySettings>>>,
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

    /// Shared HTTP client used for tracker traffic (proxy-aware).
    pub async fn http_client(&self) -> rustatio_core::reqwest::Client {
        self.http_client.read().await.clone()
    }

    /// Current proxy settings (defaults to disabled when unset).
    pub async fn proxy_settings(&self) -> ProxySettings {
        self.proxy_settings.read().await.clone().unwrap_or_default()
    }

    /// Validate, persist and apply new proxy settings to the shared client and
    /// all instances.
    pub async fn set_proxy_settings(&self, settings: ProxySettings) -> Result<(), String> {
        settings.validate()?;

        let proxy = settings.is_configured().then_some(&settings);
        let client = rustatio_core::proxy::build_client(proxy)?;

        // Persist first: if saving fails nothing is applied and the previous
        // settings stay active.
        let previous = self.proxy_settings.write().await.replace(settings);
        if let Err(err) = self.save_state().await {
            *self.proxy_settings.write().await = previous;
            return Err(err);
        }

        *self.http_client.write().await = client.clone();
        self.apply_http_client(client).await;

        Ok(())
    }

    async fn apply_http_client(&self, client: rustatio_core::reqwest::Client) {
        let fakers: Vec<Arc<RatioFakerHandle>> = {
            let instances = self.fakers.read().await;
            instances.values().map(|instance| Arc::clone(&instance.faker)).collect()
        };

        for faker in fakers {
            if let Err(err) = faker.set_http_client(Some(client.clone())).await {
                log::warn!("Failed to apply proxy settings to an instance: {err}");
            }
        }
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
        let proxy_settings = self.proxy_settings.read().await.clone();

        PersistedState {
            instances,
            next_instance_id: next_id,
            default_config,
            watch_settings,
            proxy_settings,
            version: 1,
        }
    }

    pub fn build_persisted_state_blocking(&self) -> PersistedState {
        let fakers = self.fakers.blocking_read();
        let next_id = *self.next_instance_id.blocking_read();
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

        let default_config = self.default_config.blocking_read().clone();
        let watch_settings = self.watch_settings.blocking_read().clone();
        let proxy_settings = self.proxy_settings.blocking_read().clone();

        PersistedState {
            instances,
            next_instance_id: next_id,
            default_config,
            watch_settings,
            proxy_settings,
            version: 1,
        }
    }

    pub async fn save_state(&self) -> Result<(), String> {
        let persisted = self.build_persisted_state().await;
        persistence::save_state(&persisted)
    }

    pub async fn apply_instance_config(
        &self,
        instance_id: u32,
        config: FakerConfig,
    ) -> Result<(), String> {
        let old_config = {
            let mut fakers = self.fakers.write().await;
            let instance = fakers
                .get_mut(&instance_id)
                .ok_or_else(|| format!("Instance {instance_id} not found"))?;

            let stats = instance.faker.stats_snapshot();
            if matches!(
                stats.state,
                FakerState::Running | FakerState::Starting | FakerState::Paused
            ) {
                return Err("Cannot update config while faker is running".to_string());
            }

            let old_config = instance.config.clone();
            instance
                .faker
                .update_config(config.clone(), Some(self.http_client().await))
                .await
                .map_err(|e| format!("Failed to update faker config: {e}"))?;
            instance.config = config.clone();
            old_config
        };

        if let Err(err) = self.save_state().await {
            let mut fakers = self.fakers.write().await;
            if let Some(instance) = fakers.get_mut(&instance_id) {
                instance.config = old_config.clone();
                instance
                    .faker
                    .update_config(old_config, Some(self.http_client().await))
                    .await
                    .map_err(|e| {
                        format!("{err}; rollback failed while restoring previous config: {e}")
                    })?;
            }

            return Err(err);
        }

        self.refresh_peer_listener_port().await;

        Ok(())
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
#[cfg(test)]
mod tests {
    use crate::persistence;
    use crate::state::{AppState, FakerInstance, InstanceInfo};
    use rustatio_core::{
        AppConfig, FakerConfig, PeerListenerStatus, ProxySettings, RatioFaker, RatioFakerHandle,
        TorrentInfo,
    };
    use rustatio_watch::InstanceSource;
    use std::collections::HashMap;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, OnceLock};
    use tokio::sync::{Mutex, RwLock};

    fn state_path_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn torrent() -> TorrentInfo {
        TorrentInfo {
            info_hash: [3u8; 20],
            announce: "https://tracker.test/announce".to_string(),
            announce_list: None,
            name: "saved-torrent".to_string(),
            total_size: 1024,
            piece_length: 256,
            num_pieces: 4,
            creation_date: None,
            comment: None,
            created_by: None,
            is_single_file: true,
            file_count: 1,
            files: Vec::new(),
        }
    }

    fn app_state() -> AppState {
        AppState {
            fakers: Arc::new(RwLock::new(HashMap::new())),
            next_instance_id: Arc::new(RwLock::new(1)),
            config: Arc::new(RwLock::new(AppConfig::default())),
            http_client: Arc::new(RwLock::new(rustatio_core::reqwest::Client::new())),
            proxy_settings: Arc::new(RwLock::new(None)),
            watch: Arc::new(RwLock::new(None)),
            default_config: Arc::new(RwLock::new(None)),
            watch_settings: Arc::new(RwLock::new(None)),
            should_exit: Arc::new(AtomicBool::new(false)),
            close_prompt_open: Arc::new(AtomicBool::new(false)),
            peer_listener: Arc::new(RwLock::new(None)),
            peer_listener_status: Arc::new(RwLock::new(PeerListenerStatus::default())),
        }
    }

    async fn insert_instance(state: &AppState, id: u32, config: FakerConfig) {
        let info = torrent();
        let torrent = Arc::new(info.clone());
        let summary = Arc::new(info.summary());
        let faker =
            RatioFaker::new(Arc::clone(&torrent), config.clone(), Some(state.http_client().await));
        assert!(faker.is_ok());
        let faker = faker.unwrap_or_else(|_| unreachable!());

        state.fakers.write().await.insert(
            id,
            FakerInstance {
                faker: Arc::new(RatioFakerHandle::new(faker)),
                torrent,
                summary,
                config,
                cumulative_uploaded: 0,
                cumulative_downloaded: 0,
                tags: vec!["alpha".to_string()],
                created_at: 7,
                source: InstanceSource::WatchFolder,
            },
        );
    }

    #[tokio::test]
    async fn build_summary_exposes_instance_metadata_and_source() {
        let state = app_state();
        insert_instance(&state, 5, FakerConfig::default()).await;

        let fakers = state.fakers.read().await;
        assert!(fakers.contains_key(&5));
        let Some(instance) = fakers.get(&5) else {
            return;
        };

        let summary =
            instance.faker.with_stats(|stats| crate::state::build_summary(5, instance, stats));

        assert_eq!(summary.id, "5");
        assert_eq!(summary.name, "saved-torrent");
        assert_eq!(summary.info_hash, "0303030303030303030303030303030303030303");
        assert_eq!(summary.primary_tracker_host.as_deref(), Some("tracker.test"));
        assert_eq!(summary.state, "stopped");
        assert_eq!(summary.source, "watch_folder");
        assert_eq!(summary.tags, vec!["alpha".to_string()]);
        assert_eq!(summary.total_size, 1024);
        assert_eq!(summary.created_at, 7);
    }

    #[tokio::test]
    async fn build_persisted_state_keeps_instance_config_as_source_of_truth() {
        let state = app_state();
        let config = FakerConfig {
            upload_rate: 321.0,
            port: 51413,
            stop_at_ratio: Some(3.5),
            ..FakerConfig::default()
        };

        insert_instance(&state, 5, config.clone()).await;

        let persisted = state.build_persisted_state().await;
        assert_eq!(persisted.instances.len(), 1);

        let saved = persisted.instances.get(&5);
        assert!(saved.is_some());
        let saved = saved.unwrap_or_else(|| unreachable!());
        assert_eq!(saved.config.upload_rate, config.upload_rate);
        assert_eq!(saved.config.port, config.port);
        assert_eq!(saved.config.stop_at_ratio, config.stop_at_ratio);
        assert!(saved.from_watch_folder);
        assert_eq!(saved.tags, vec!["alpha".to_string()]);
    }

    #[tokio::test]
    async fn build_persisted_state_blocking_matches_async_snapshot() {
        let state = app_state();
        let config = FakerConfig {
            upload_rate: 321.0,
            port: 51413,
            stop_at_ratio: Some(3.5),
            ..FakerConfig::default()
        };

        insert_instance(&state, 5, config).await;

        let async_state = state.build_persisted_state().await;
        let blocking_state = tokio::task::spawn_blocking({
            let state = state.clone();
            move || state.build_persisted_state_blocking()
        })
        .await;
        assert!(blocking_state.is_ok());
        let blocking_state = blocking_state.unwrap_or_else(|_| unreachable!());

        assert_eq!(blocking_state.next_instance_id, async_state.next_instance_id);
        assert_eq!(blocking_state.version, async_state.version);

        let async_default = serde_json::to_value(async_state.default_config);
        assert!(async_default.is_ok());
        let async_default = async_default.unwrap_or_else(|_| unreachable!());

        let blocking_default = serde_json::to_value(blocking_state.default_config);
        assert!(blocking_default.is_ok());
        let blocking_default = blocking_default.unwrap_or_else(|_| unreachable!());
        assert_eq!(blocking_default, async_default);

        let async_watch = serde_json::to_value(async_state.watch_settings);
        assert!(async_watch.is_ok());
        let async_watch = async_watch.unwrap_or_else(|_| unreachable!());

        let blocking_watch = serde_json::to_value(blocking_state.watch_settings);
        assert!(blocking_watch.is_ok());
        let blocking_watch = blocking_watch.unwrap_or_else(|_| unreachable!());
        assert_eq!(blocking_watch, async_watch);

        let async_instance = async_state.instances.get(&5);
        assert!(async_instance.is_some());
        let async_instance = async_instance.unwrap_or_else(|| unreachable!());

        let blocking_instance = blocking_state.instances.get(&5);
        assert!(blocking_instance.is_some());
        let blocking_instance = blocking_instance.unwrap_or_else(|| unreachable!());

        assert_eq!(blocking_instance.id, async_instance.id);
        assert_eq!(blocking_instance.cumulative_uploaded, async_instance.cumulative_uploaded);
        assert_eq!(blocking_instance.cumulative_downloaded, async_instance.cumulative_downloaded);
        assert_eq!(blocking_instance.created_at, async_instance.created_at);
        assert_eq!(blocking_instance.tags, async_instance.tags);
        assert_eq!(blocking_instance.from_watch_folder, async_instance.from_watch_folder);
        assert!(blocking_instance.updated_at >= async_instance.updated_at);
        assert!(blocking_instance.updated_at <= async_instance.updated_at.saturating_add(1));

        let async_torrent = serde_json::to_value(&async_instance.torrent);
        assert!(async_torrent.is_ok());
        let async_torrent = async_torrent.unwrap_or_else(|_| unreachable!());

        let blocking_torrent = serde_json::to_value(&blocking_instance.torrent);
        assert!(blocking_torrent.is_ok());
        let blocking_torrent = blocking_torrent.unwrap_or_else(|_| unreachable!());
        assert_eq!(blocking_torrent, async_torrent);

        let async_config = serde_json::to_value(&async_instance.config);
        assert!(async_config.is_ok());
        let async_config = async_config.unwrap_or_else(|_| unreachable!());

        let blocking_config = serde_json::to_value(&blocking_instance.config);
        assert!(blocking_config.is_ok());
        let blocking_config = blocking_config.unwrap_or_else(|_| unreachable!());
        assert_eq!(blocking_config, async_config);

        let async_state_value = serde_json::to_value(async_instance.state);
        assert!(async_state_value.is_ok());
        let async_state_value = async_state_value.unwrap_or_else(|_| unreachable!());

        let blocking_state_value = serde_json::to_value(blocking_instance.state);
        assert!(blocking_state_value.is_ok());
        let blocking_state_value = blocking_state_value.unwrap_or_else(|_| unreachable!());
        assert_eq!(blocking_state_value, async_state_value);
    }

    #[test]
    fn instance_info_serializes_server_style_shape() {
        let info = InstanceInfo {
            id: "5".to_string(),
            torrent: torrent().summary(),
            config: FakerConfig::default(),
            stats: RatioFaker::stats_from_config(&FakerConfig::default()),
            created_at: 7,
            source: "watch_folder".to_string(),
            tags: vec!["alpha".to_string()],
        };

        let json = serde_json::to_value(info);
        assert!(json.is_ok());
        let json = json.unwrap_or_else(|_| unreachable!());

        assert_eq!(json.get("id").and_then(serde_json::Value::as_str), Some("5"));
        assert!(json.get("torrent").is_some());
        assert!(json.get("config").is_some());
        assert!(json.get("stats").is_some());
        assert_eq!(json.get("source").and_then(serde_json::Value::as_str), Some("watch_folder"));
    }

    #[tokio::test]
    async fn apply_instance_config_saves_updated_state_immediately() {
        let _guard = state_path_test_lock().lock().await;
        let temp = tempfile::tempdir();
        assert!(temp.is_ok());
        let temp = temp.unwrap_or_else(|_| unreachable!());
        let path = temp.path().join("desktop-state.json");
        persistence::set_test_state_file_path(Some(path));

        let state = app_state();
        insert_instance(&state, 5, FakerConfig::default()).await;

        let updated = FakerConfig {
            upload_rate: 444.0,
            download_rate: 222.0,
            port: 51413,
            stop_at_ratio: Some(4.0),
            ..FakerConfig::default()
        };

        let result = state.apply_instance_config(5, updated.clone()).await;
        assert!(result.is_ok());

        let persisted = persistence::load_state();
        let saved = persisted.instances.get(&5);
        assert!(saved.is_some());
        let saved = saved.unwrap_or_else(|| unreachable!());
        assert_eq!(saved.config.upload_rate, updated.upload_rate);
        assert_eq!(saved.config.port, updated.port);
        assert_eq!(saved.config.stop_at_ratio, updated.stop_at_ratio);

        persistence::set_test_state_file_path(None);
    }

    #[tokio::test]
    async fn apply_instance_config_rolls_back_when_save_fails() {
        let _guard = state_path_test_lock().lock().await;
        let temp = tempfile::tempdir();
        assert!(temp.is_ok());
        let temp = temp.unwrap_or_else(|_| unreachable!());
        let blocker = temp.path().join("blocked-parent");
        let blocker_write = std::fs::write(&blocker, b"not a directory");
        assert!(blocker_write.is_ok());
        let path = blocker.join("desktop-state.json");
        persistence::set_test_state_file_path(Some(path));

        let state = app_state();
        let original = FakerConfig { upload_rate: 111.0, port: 40000, ..FakerConfig::default() };
        insert_instance(&state, 5, original.clone()).await;

        let updated = FakerConfig { upload_rate: 444.0, port: 51413, ..FakerConfig::default() };

        let result = state.apply_instance_config(5, updated).await;
        assert!(result.is_err());

        let fakers = state.fakers.read().await;
        let instance = fakers.get(&5);
        assert!(instance.is_some());
        let instance = instance.unwrap_or_else(|| unreachable!());
        assert_eq!(instance.config.upload_rate, original.upload_rate);
        assert_eq!(instance.config.port, original.port);
        assert_eq!(instance.faker.effective_port().await, original.port);

        persistence::set_test_state_file_path(None);
    }

    #[tokio::test]
    async fn save_state_persists_bulk_style_config_changes() {
        let _guard = state_path_test_lock().lock().await;
        let temp = tempfile::tempdir();
        assert!(temp.is_ok());
        let temp = temp.unwrap_or_else(|_| unreachable!());
        let path = temp.path().join("desktop-state.json");
        persistence::set_test_state_file_path(Some(path));

        let state = app_state();
        let updated = FakerConfig {
            upload_rate: 777.0,
            port: 42424,
            stop_at_ratio: Some(5.0),
            ..FakerConfig::default()
        };

        insert_instance(&state, 5, FakerConfig::default()).await;

        {
            let mut fakers = state.fakers.write().await;
            let instance = fakers.get_mut(&5);
            assert!(instance.is_some());
            let instance = instance.unwrap_or_else(|| unreachable!());
            instance.config = updated.clone();
        }

        let saved = state.save_state().await;
        assert!(saved.is_ok());

        let persisted = persistence::load_state();
        let saved = persisted.instances.get(&5);
        assert!(saved.is_some());
        let saved = saved.unwrap_or_else(|| unreachable!());
        assert_eq!(saved.config.upload_rate, updated.upload_rate);
        assert_eq!(saved.config.port, updated.port);
        assert_eq!(saved.config.stop_at_ratio, updated.stop_at_ratio);

        persistence::set_test_state_file_path(None);
    }

    #[tokio::test]
    async fn proxy_settings_are_included_in_persisted_state() {
        let state = app_state();
        let settings = ProxySettings {
            enabled: true,
            url: "socks5h://127.0.0.1:1080".to_string(),
            ..ProxySettings::default()
        };
        *state.proxy_settings.write().await = Some(settings.clone());

        let persisted = state.build_persisted_state().await;
        assert_eq!(persisted.proxy_settings, Some(settings.clone()));

        let blocking = tokio::task::spawn_blocking({
            let state = state.clone();
            move || state.build_persisted_state_blocking()
        })
        .await;
        assert!(blocking.is_ok());
        let blocking = blocking.unwrap_or_else(|_| unreachable!());
        assert_eq!(blocking.proxy_settings, Some(settings));
    }

    #[tokio::test]
    async fn set_proxy_settings_rejects_invalid_url() {
        let state = app_state();
        let invalid = ProxySettings {
            enabled: true,
            url: "ftp://proxy:21".to_string(),
            ..ProxySettings::default()
        };

        assert!(state.set_proxy_settings(invalid).await.is_err());
        assert_eq!(state.proxy_settings().await, ProxySettings::default());
    }
}
