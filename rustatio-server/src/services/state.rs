use super::events::{EventBroadcaster, InstanceEvent, LogEvent};
use super::instance::{FakerInstance, InstanceInfo};
use super::lifecycle::InstanceLifecycle;
use super::persistence::{
    now_timestamp, CustomPreset, DefaultPreset, InstanceSource, PersistedInstance,
    PersistedRuntime, PersistedState, Persistence, WatchSettings,
};
use rustatio_core::logger::set_instance_context_str;
use rustatio_core::{
    FakerConfig, FakerState, FakerStats, InstanceSummary, PeerListenerService, PeerListenerStatus,
    RatioFaker, RatioFakerHandle, TorrentInfo, TorrentSummary,
};
use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex, RwLock};

type PeerListenerHandle = Arc<Mutex<PeerListenerService>>;

#[derive(Clone)]
pub struct AppState {
    pub instances: Arc<RwLock<HashMap<String, FakerInstance>>>,
    pub log_sender: broadcast::Sender<LogEvent>,
    pub instance_sender: broadcast::Sender<InstanceEvent>,
    persistence: Arc<Persistence>,
    default_config: Arc<RwLock<Option<FakerConfig>>>,
    default_preset: Arc<RwLock<Option<DefaultPreset>>>,
    watch_settings: Arc<RwLock<Option<WatchSettings>>>,
    custom_presets: Arc<RwLock<Vec<CustomPreset>>>,
    http_client: reqwest::Client,
    forwarded_port: Arc<AtomicU16>,
    server_vpn_port_sync: bool,
    peer_listener: Arc<RwLock<Option<PeerListenerHandle>>>,
    peer_listener_status: Arc<RwLock<PeerListenerStatus>>,
}

pub struct InstanceBuildContext {
    id: String,
    torrent: Arc<TorrentInfo>,
    summary: Arc<TorrentSummary>,
    config: FakerConfig,
    source: InstanceSource,
}

impl InstanceBuildContext {
    pub fn new(
        id: &str,
        torrent: TorrentInfo,
        config: FakerConfig,
        source: InstanceSource,
    ) -> Self {
        let summary = Arc::new(torrent.summary());
        Self { id: id.to_string(), torrent: Arc::new(torrent), summary, config, source }
    }
}

struct ExistingInstanceState {
    cumulative_uploaded: u64,
    cumulative_downloaded: u64,
    created_at: u64,
    source: InstanceSource,
    tags: Vec<String>,
    completion_percent: Option<f64>,
}

impl AppState {
    pub fn new(data_dir: &str) -> Self {
        let (log_sender, _) = broadcast::channel(256);
        let (instance_sender, _) = broadcast::channel(1024);
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            log_sender,
            instance_sender,
            persistence: Arc::new(Persistence::new(data_dir)),
            default_config: Arc::new(RwLock::new(None)),
            default_preset: Arc::new(RwLock::new(None)),
            watch_settings: Arc::new(RwLock::new(None)),
            custom_presets: Arc::new(RwLock::new(Vec::new())),
            http_client: reqwest::Client::new(),
            forwarded_port: Arc::new(AtomicU16::new(0)),
            server_vpn_port_sync: std::env::var("VPN_PORT_SYNC")
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
                .unwrap_or(false),
            peer_listener: Arc::new(RwLock::new(None)),
            peer_listener_status: Arc::new(RwLock::new(PeerListenerStatus::default())),
        }
    }

    pub fn current_forwarded_port(&self) -> Option<u16> {
        match self.forwarded_port.load(Ordering::Relaxed) {
            0 => None,
            port => Some(port),
        }
    }

    pub fn set_current_forwarded_port(&self, port: Option<u16>) {
        self.forwarded_port.store(port.unwrap_or(0), Ordering::Relaxed);
    }

    pub const fn vpn_port_sync_enabled(&self) -> bool {
        self.server_vpn_port_sync
    }

    pub async fn peer_listener_status(&self) -> PeerListenerStatus {
        self.peer_listener_status.read().await.clone()
    }

    pub async fn set_peer_listener_status(&self, status: PeerListenerStatus) {
        *self.peer_listener_status.write().await = status;
    }

    pub async fn attach_peer_listener(&self, listener: PeerListenerHandle) {
        *self.peer_listener.write().await = Some(listener);
        self.refresh_peer_listener_port().await;
    }

    async fn desired_peer_port_from_instances(&self) -> Result<Option<u16>, String> {
        let instances = self.instances.read().await;
        let mut manual_ports = BTreeSet::new();
        let mut has_synced_active = false;

        for instance in instances.values() {
            let state = instance.faker.stats_snapshot().state;
            if matches!(state, FakerState::Starting | FakerState::Running | FakerState::Paused) {
                if instance.config.vpn_port_sync {
                    has_synced_active = true;
                } else {
                    manual_ports.insert(instance.config.port);
                }
            }
        }

        if has_synced_active {
            if let Some(port) = self.current_forwarded_port() {
                if manual_ports.is_empty() {
                    return Ok(Some(port));
                }

                return Err(format!(
                    "multiple active peer ports configured: {}, {}",
                    port,
                    manual_ports.iter().map(u16::to_string).collect::<Vec<_>>().join(", ")
                ));
            }
        }

        match manual_ports.len() {
            0 => Ok(None),
            1 => Ok(manual_ports.iter().next().copied()),
            _ => Err(format!(
                "multiple active peer ports configured: {}",
                manual_ports.iter().map(u16::to_string).collect::<Vec<_>>().join(", ")
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
                tracing::warn!("Peer listener disabled: {err}");
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

    fn apply_forwarded_port_to_config(&self, config: &mut FakerConfig) {
        if config.vpn_port_sync {
            if let Some(port) = self.current_forwarded_port() {
                config.port = port;
            }
        }
    }

    const fn apply_cumulative_totals(config: &mut FakerConfig, uploaded: u64, downloaded: u64) {
        config.initial_uploaded = uploaded;
        config.initial_downloaded = downloaded;
    }

    pub async fn get_default_config(&self) -> Option<FakerConfig> {
        self.default_config.read().await.clone()
    }

    pub async fn get_default_preset(&self) -> Option<DefaultPreset> {
        self.default_preset.read().await.clone()
    }

    pub async fn get_effective_default_config(&self) -> FakerConfig {
        let mut config = self.get_default_config().await.unwrap_or_else(|| FakerConfig {
            vpn_port_sync: self.server_vpn_port_sync,
            ..FakerConfig::default()
        });
        self.apply_forwarded_port_to_config(&mut config);
        config
    }

    pub async fn set_default_config(&self, config: Option<FakerConfig>) -> Result<(), String> {
        *self.default_config.write().await = config.clone();
        *self.default_preset.write().await = None;
        self.save_state().await
    }

    pub async fn set_default_preset(&self, preset: Option<DefaultPreset>) -> Result<(), String> {
        *self.default_preset.write().await = preset.clone();
        *self.default_config.write().await = preset.clone().map(|value| value.settings.into());
        self.save_state().await
    }

    pub async fn get_watch_settings_optional(&self) -> Option<WatchSettings> {
        self.watch_settings.read().await.clone()
    }

    pub async fn list_custom_presets(&self) -> Vec<CustomPreset> {
        self.custom_presets.read().await.clone()
    }

    pub async fn upsert_custom_preset(&self, preset: CustomPreset) -> Result<(), String> {
        let mut presets = self.custom_presets.write().await;

        if let Some(existing) = presets.iter_mut().find(|item| item.id == preset.id) {
            *existing = preset;
        } else {
            presets.push(preset);
        }

        drop(presets);
        self.save_state().await
    }

    pub async fn delete_custom_preset(&self, id: &str) -> Result<(), String> {
        let mut presets = self.custom_presets.write().await;
        let original_len = presets.len();
        presets.retain(|preset| preset.id != id);
        let changed = presets.len() != original_len;
        drop(presets);

        if changed {
            self.save_state().await?;
        }

        Ok(())
    }

    pub async fn set_watch_settings(&self, settings: WatchSettings) -> Result<(), String> {
        *self.watch_settings.write().await = Some(settings.clone());
        self.save_state().await
    }

    pub async fn load_saved_state(&self) -> Result<usize, String> {
        let saved = self.persistence.load().await;

        if let Some(config) = saved.default_config.clone() {
            *self.default_config.write().await = Some(config);
            tracing::info!("Restored default config from saved state");
        }

        if let Some(preset) = saved.default_preset.clone() {
            *self.default_preset.write().await = Some(preset);
        }

        if let Some(settings) = saved.watch_settings.clone() {
            *self.watch_settings.write().await = Some(settings);
            tracing::info!("Restored watch settings from saved state");
        }

        if !saved.custom_presets.is_empty() {
            *self.custom_presets.write().await = saved.custom_presets.clone();
            tracing::info!(
                "Restored {} custom preset(s) from saved state",
                saved.custom_presets.len()
            );
        }

        let mut restored_count = 0;
        let mut auto_start_ids = Vec::new();

        // First pass: insert all instances so they appear immediately in the UI
        for (id, persisted) in &saved.instances {
            tracing::info!(
                "Restoring instance {} ({}) - state: {:?}",
                id,
                persisted.torrent.name,
                persisted.state
            );

            let mut faker_config = persisted.config.clone();
            let runtime = persisted.runtime.as_ref();
            faker_config.initial_uploaded =
                runtime.map_or(persisted.cumulative_uploaded, |rt| rt.uploaded);
            faker_config.initial_downloaded =
                runtime.map_or(persisted.cumulative_downloaded, |rt| rt.downloaded);

            let summary = Arc::new(persisted.torrent.clone());
            let torrent = Arc::new(persisted.torrent.to_info());

            match RatioFaker::new(
                Arc::clone(&torrent),
                faker_config,
                Some(self.http_client.clone()),
            ) {
                Ok(faker) => {
                    let restored_stats = runtime.map_or_else(
                        || Self::default_runtime_stats(&persisted.config),
                        |value| {
                            Self::stats_from_runtime(
                                value,
                                persisted.state,
                                persisted.config.post_stop_action,
                            )
                        },
                    );

                    let instance = FakerInstance {
                        faker: Arc::new(RatioFakerHandle::new(faker)),
                        torrent,
                        summary,
                        config: persisted.config.clone(),
                        torrent_info_hash: persisted.torrent.info_hash,
                        cumulative_uploaded: persisted.cumulative_uploaded,
                        cumulative_downloaded: persisted.cumulative_downloaded,
                        created_at: persisted.created_at,
                        source: persisted.source,
                        tags: persisted.tags.clone(),
                    };

                    instance.faker.restore_snapshot(restored_stats).await;

                    self.instances.write().await.insert(id.clone(), instance);

                    self.emit_instance_event(InstanceEvent::Created {
                        id: id.clone(),
                        torrent_name: persisted.torrent.name.clone(),
                        info_hash: hex::encode(persisted.torrent.info_hash),
                        auto_started: false,
                    });

                    if matches!(persisted.state, FakerState::Starting | FakerState::Running) {
                        auto_start_ids.push(id.clone());
                    }

                    restored_count += 1;
                }
                Err(e) => {
                    tracing::error!("Failed to restore instance {}: {}", id, e);
                }
            }
        }

        if restored_count > 0 {
            tracing::info!("Restored {} instances from saved state", restored_count);
        }

        // Second pass: auto-start instances that were previously running
        if !auto_start_ids.is_empty() {
            tracing::info!("Auto-starting {} instance(s)...", auto_start_ids.len());
            for id in &auto_start_ids {
                if let Err(e) = self.start_instance(id).await {
                    tracing::warn!("Failed to auto-start instance {}: {}", id, e);
                }
            }
        }

        Ok(restored_count)
    }

    pub async fn save_state(&self) -> Result<(), String> {
        let instances = self.instances.read().await;

        let default_config = self.default_config.read().await.clone();
        let default_preset = self.default_preset.read().await.clone();
        let watch_settings = self.watch_settings.read().await.clone();
        let custom_presets = self.custom_presets.read().await.clone();

        let mut persisted = PersistedState {
            instances: HashMap::new(),
            default_config,
            default_preset,
            watch_settings,
            custom_presets,
            version: 1,
        };

        for (id, instance) in instances.iter() {
            let stats = instance.faker.stats_snapshot();
            let mut config = instance.config.clone();
            config.completion_percent = stats.torrent_completion;

            persisted.instances.insert(
                id.clone(),
                PersistedInstance {
                    id: id.clone(),
                    torrent: (*instance.summary).clone(),
                    config,
                    cumulative_uploaded: stats.uploaded,
                    cumulative_downloaded: stats.downloaded,
                    state: stats.state,
                    created_at: instance.created_at,
                    updated_at: now_timestamp(),
                    source: instance.source,
                    tags: instance.tags.clone(),
                    runtime: Some(Self::runtime_from_stats(&stats)),
                },
            );
        }

        self.persistence.save(&persisted).await
    }

    #[allow(clippy::unused_self)]
    pub fn next_instance_id(&self) -> String {
        nanoid::nanoid!(10)
    }

    pub async fn instance_exists(&self, id: &str) -> bool {
        self.instances.read().await.contains_key(id)
    }

    pub async fn update_instance_config(
        &self,
        id: &str,
        config: FakerConfig,
    ) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;
        let mut config = config;
        self.apply_forwarded_port_to_config(&mut config);
        let mut faker_config = config.clone();
        Self::apply_cumulative_totals(
            &mut faker_config,
            instance.cumulative_uploaded,
            instance.cumulative_downloaded,
        );

        instance
            .faker
            .update_config(faker_config, Some(self.http_client.clone()))
            .await
            .map_err(|e| e.to_string())?;
        instance.config = config;
        drop(instances);
        self.refresh_peer_listener_port().await;

        Ok(())
    }

    pub async fn update_instance_config_only(
        &self,
        id: &str,
        config: FakerConfig,
    ) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;
        let mut config = config;
        self.apply_forwarded_port_to_config(&mut config);
        instance.config = config.clone();

        instance
            .faker
            .update_config(config, Some(self.http_client.clone()))
            .await
            .map_err(|e| format!("Failed to update faker config: {e}"))?;
        drop(instances);

        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after updating instance config: {}", e);
        }

        self.refresh_peer_listener_port().await;

        Ok(())
    }

    pub async fn bulk_update_configs(
        &self,
        entries: Vec<(String, FakerConfig)>,
    ) -> (Vec<String>, Vec<(String, String)>) {
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();
        let mut instances = self.instances.write().await;

        for (id, config) in entries {
            match instances.get_mut(&id) {
                Some(instance) => {
                    let mut config = config;
                    self.apply_forwarded_port_to_config(&mut config);

                    let mut faker_config = config.clone();
                    Self::apply_cumulative_totals(
                        &mut faker_config,
                        instance.cumulative_uploaded,
                        instance.cumulative_downloaded,
                    );

                    let result = instance
                        .faker
                        .update_config(faker_config, Some(self.http_client.clone()))
                        .await;
                    match result {
                        Ok(()) => {
                            instance.config = config;
                            succeeded.push(id);
                        }
                        Err(e) => {
                            failed.push((id, e.to_string()));
                        }
                    }
                }
                None => {
                    failed.push((id.clone(), format!("Instance {id} not found")));
                }
            }
        }

        drop(instances);
        self.refresh_peer_listener_port().await;

        (succeeded, failed)
    }

    pub async fn create_instance(
        &self,
        id: &str,
        torrent: TorrentInfo,
        config: FakerConfig,
    ) -> Result<(), String> {
        let mut config = config;
        self.apply_forwarded_port_to_config(&mut config);
        let context = InstanceBuildContext::new(id, torrent, config, InstanceSource::Manual);
        self.create_instance_internal(context).await
    }

    pub async fn create_idle_instance(&self, id: &str, torrent: TorrentInfo) -> Result<(), String> {
        let config = self.get_effective_default_config().await;
        let context = InstanceBuildContext::new(id, torrent, config, InstanceSource::Manual);
        let torrent = Arc::clone(&context.torrent);
        self.create_instance_internal(context).await?;

        self.emit_instance_event(InstanceEvent::Created {
            id: id.to_string(),
            torrent_name: torrent.name.clone(),
            info_hash: hex::encode(torrent.info_hash),
            auto_started: false,
        });

        Ok(())
    }

    pub async fn create_instance_with_event(
        &self,
        id: &str,
        torrent: TorrentInfo,
        config: FakerConfig,
        auto_started: bool,
    ) -> Result<(), String> {
        let mut config = config;
        self.apply_forwarded_port_to_config(&mut config);
        let context = InstanceBuildContext::new(id, torrent, config, InstanceSource::WatchFolder);
        let torrent = Arc::clone(&context.torrent);
        self.create_instance_internal(context).await?;

        self.emit_instance_event(InstanceEvent::Created {
            id: id.to_string(),
            torrent_name: torrent.name.clone(),
            info_hash: hex::encode(torrent.info_hash),
            auto_started,
        });

        Ok(())
    }

    async fn create_instance_internal(&self, context: InstanceBuildContext) -> Result<(), String> {
        set_instance_context_str(Some(&context.id));

        let id = context.id.clone();
        let existing = self.collect_existing_instance_state(&context).await;
        let faker_config = Self::build_faker_config(&context, &existing);
        let instance = self.build_instance(context, faker_config, existing)?;
        self.insert_instance(id, instance).await
    }

    pub async fn get_stats(&self, id: &str) -> Result<FakerStats, String> {
        let faker = {
            let instances = self.instances.read().await;
            let instance = instances.get(id).ok_or("Instance not found")?;
            Arc::clone(&instance.faker)
        };
        let stats = faker.stats_snapshot();
        Ok(stats)
    }

    pub async fn get_instance_torrent(&self, id: &str) -> Result<TorrentInfo, String> {
        let instances = self.instances.read().await;
        let instance = instances.get(id).ok_or("Instance not found")?;
        Ok((*instance.torrent).clone())
    }

    pub async fn get_instance_summary(&self, id: &str) -> Result<TorrentSummary, String> {
        let instances = self.instances.read().await;
        let instance = instances.get(id).ok_or("Instance not found")?;
        Ok((*instance.summary).clone())
    }

    pub async fn delete_instance(&self, id: &str, force: bool) -> Result<(), String> {
        if !force {
            let instances = self.instances.read().await;
            if let Some(instance) = instances.get(id) {
                if instance.source == InstanceSource::WatchFolder {
                    return Err(
                        "Cannot delete watch folder instance. Delete the torrent file from the watch folder instead, or use force delete."
                            .to_string(),
                    );
                }
            }
        }

        // Stop the faker if running before removing
        {
            let instances = self.instances.read().await;
            if let Some(instance) = instances.get(id) {
                let _ = instance.faker.stop().await;
            }
        }

        let removed = self.instances.write().await.remove(id);

        if removed.is_some() {
            self.emit_instance_event(InstanceEvent::Deleted { id: id.to_string() });
        }

        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after deleting instance: {}", e);
        }

        self.refresh_peer_listener_port().await;

        Ok(())
    }

    pub async fn list_instances(&self) -> Vec<InstanceInfo> {
        let instances = self.instances.read().await;
        let mut result = Vec::new();

        for (id, instance) in instances.iter() {
            let stats = instance.faker.stats_snapshot();

            result.push(InstanceInfo {
                id: id.clone(),
                torrent: Arc::clone(&instance.summary),
                config: instance.config.clone(),
                stats,
                created_at: instance.created_at,
                source: instance.source,
                tags: instance.tags.clone(),
            });
        }

        result
    }

    pub async fn apply_vpn_forwarded_port(&self, port: u16) -> Result<usize, String> {
        self.set_current_forwarded_port(Some(port));

        let mut instances = self.instances.write().await;
        let mut updated = 0usize;

        for instance in instances.values_mut() {
            if !instance.config.vpn_port_sync || instance.config.port == port {
                continue;
            }

            let mut config = instance.config.clone();
            config.port = port;

            let mut faker_config = config.clone();
            Self::apply_cumulative_totals(
                &mut faker_config,
                instance.cumulative_uploaded,
                instance.cumulative_downloaded,
            );

            let state = instance.faker.stats_snapshot().state;
            let is_running =
                matches!(state, FakerState::Starting | FakerState::Running | FakerState::Paused);

            if is_running {
                instance.faker.set_runtime_port(port).await;
            } else {
                instance
                    .faker
                    .update_config(faker_config, Some(self.http_client.clone()))
                    .await
                    .map_err(|e| format!("Failed to update synced port: {e}"))?;
            }

            instance.config = config;
            updated += 1;
        }

        drop(instances);
        self.refresh_peer_listener_port().await;

        Ok(updated)
    }

    pub async fn get_instance_info_for_delete(
        &self,
        id: &str,
    ) -> Option<(InstanceSource, [u8; 20])> {
        let instances = self.instances.read().await;
        instances.get(id).map(|inst| (inst.source, inst.torrent_info_hash))
    }

    pub async fn find_instance_by_info_hash(&self, info_hash: &[u8; 20]) -> Option<String> {
        let instances = self.instances.read().await;
        for (id, instance) in instances.iter() {
            if &instance.torrent_info_hash == info_hash {
                return Some(id.clone());
            }
        }
        None
    }

    pub async fn update_instance_source(
        &self,
        id: &str,
        source: InstanceSource,
    ) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;
        instance.source = source;
        drop(instances);

        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after updating instance source: {}", e);
        }

        Ok(())
    }

    pub async fn update_instance_source_by_info_hash(
        &self,
        info_hash: &[u8; 20],
        source: InstanceSource,
    ) -> Result<(), String> {
        let Some(id) = self.find_instance_by_info_hash(info_hash).await else {
            return Ok(());
        };
        self.update_instance_source(&id, source).await
    }

    pub async fn update_instance_tags(&self, id: &str, tags: Vec<String>) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;
        instance.tags = tags;
        drop(instances);

        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after updating tags: {}", e);
        }
        Ok(())
    }

    pub async fn grid_update_tags(
        &self,
        ids: &[String],
        add_tags: &[String],
        remove_tags: &[String],
    ) -> Result<usize, String> {
        let mut instances = self.instances.write().await;
        let mut updated = 0;

        for id in ids {
            if let Some(instance) = instances.get_mut(id) {
                for tag in add_tags {
                    if !instance.tags.contains(tag) {
                        instance.tags.push(tag.clone());
                    }
                }
                instance.tags.retain(|t| !remove_tags.contains(t));
                updated += 1;
            }
        }

        drop(instances);
        if updated > 0 {
            if let Err(e) = self.save_state().await {
                tracing::warn!("Failed to save state after grid tag update: {}", e);
            }
        }
        Ok(updated)
    }

    pub async fn list_instance_summaries(&self) -> Vec<InstanceSummary> {
        let instances = self.instances.read().await;
        let mut result = Vec::with_capacity(instances.len());

        for (id, instance) in instances.iter() {
            let stats = instance.faker.stats_snapshot();

            let source = match instance.source {
                InstanceSource::Manual => "manual",
                InstanceSource::WatchFolder => "watch_folder",
            };

            let state = match stats.state {
                FakerState::Paused => "paused",
                _ if stats.is_idling => "idle",
                FakerState::Idle => "idle",
                FakerState::Starting => "starting",
                FakerState::Running => "running",
                FakerState::Stopping => "stopping",
                FakerState::Stopped => "stopped",
            };

            result.push(InstanceSummary {
                id: id.clone(),
                name: instance.summary.name.clone(),
                info_hash: hex::encode(instance.torrent_info_hash),
                state: state.to_string(),
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
                source: source.to_string(),
                created_at: instance.created_at,
            });
        }

        result
    }

    pub async fn create_instance_with_tags(
        &self,
        context: InstanceBuildContext,
        tags: Vec<String>,
    ) -> Result<(), String> {
        let mut context = context;
        self.apply_forwarded_port_to_config(&mut context.config);
        let id = context.id.clone();
        self.create_instance_internal(context).await?;

        if !tags.is_empty() {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(&id) {
                instance.tags = tags;
            }
        }

        Ok(())
    }

    async fn collect_existing_instance_state(
        &self,
        context: &InstanceBuildContext,
    ) -> ExistingInstanceState {
        let torrent_info_hash = context.torrent.info_hash;
        let instances = self.instances.read().await;
        if let Some(existing) = instances.get(&context.id) {
            if existing.torrent_info_hash == torrent_info_hash {
                let stats = existing.faker.stats_snapshot();
                return ExistingInstanceState {
                    cumulative_uploaded: existing.cumulative_uploaded,
                    cumulative_downloaded: existing.cumulative_downloaded,
                    created_at: existing.created_at,
                    source: existing.source,
                    tags: existing.tags.clone(),
                    completion_percent: Some(stats.torrent_completion),
                };
            }
        }

        ExistingInstanceState {
            cumulative_uploaded: 0,
            cumulative_downloaded: 0,
            created_at: now_timestamp(),
            source: context.source,
            tags: Vec::new(),
            completion_percent: None,
        }
    }

    fn build_faker_config(
        context: &InstanceBuildContext,
        existing: &ExistingInstanceState,
    ) -> FakerConfig {
        let mut faker_config = context.config.clone();
        Self::apply_cumulative_totals(
            &mut faker_config,
            existing.cumulative_uploaded,
            existing.cumulative_downloaded,
        );
        if let Some(completion) = existing.completion_percent {
            faker_config.completion_percent = completion;
        }
        faker_config
    }

    fn build_instance(
        &self,
        context: InstanceBuildContext,
        faker_config: FakerConfig,
        existing: ExistingInstanceState,
    ) -> Result<FakerInstance, String> {
        let torrent_info_hash = context.torrent.info_hash;
        let faker = RatioFaker::new(
            Arc::clone(&context.torrent),
            faker_config,
            Some(self.http_client.clone()),
        )
        .map_err(|e| e.to_string())?;

        Ok(FakerInstance {
            faker: Arc::new(RatioFakerHandle::new(faker)),
            torrent: context.torrent,
            summary: context.summary,
            config: context.config,
            torrent_info_hash,
            cumulative_uploaded: existing.cumulative_uploaded,
            cumulative_downloaded: existing.cumulative_downloaded,
            created_at: existing.created_at,
            source: existing.source,
            tags: existing.tags,
        })
    }

    async fn insert_instance(&self, id: String, instance: FakerInstance) -> Result<(), String> {
        self.instances.write().await.insert(id, instance);

        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after creating instance: {}", e);
        }

        Ok(())
    }

    pub async fn delete_instance_by_info_hash(&self, info_hash: &[u8; 20]) -> Result<(), String> {
        let Some(id) = self.find_instance_by_info_hash(info_hash).await else {
            return Ok(());
        };

        // Stop the faker if running before removing
        {
            let instances = self.instances.read().await;
            if let Some(instance) = instances.get(&id) {
                let _ = instance.faker.stop().await;
            }
        }

        let removed = self.instances.write().await.remove(&id);

        if removed.is_some() {
            tracing::info!("Deleted instance {} (torrent file removed from watch folder)", id);
            self.emit_instance_event(InstanceEvent::Deleted { id: id.clone() });
        }

        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after deleting instance: {}", e);
        }

        self.refresh_peer_listener_port().await;

        Ok(())
    }

    pub async fn shutdown_all(&self) {
        tracing::info!("Stopping all running faker instances...");

        let instances = self.instances.read().await;
        for (id, instance) in instances.iter() {
            let stats = instance.faker.stats_snapshot();
            if matches!(
                stats.state,
                FakerState::Starting | FakerState::Running | FakerState::Paused
            ) {
                if let Err(e) = instance.faker.stop().await {
                    tracing::warn!("Failed to stop instance {}: {}", id, e);
                }
            }
        }
        drop(instances);

        tracing::info!("All faker instances stopped");
        self.refresh_peer_listener_port().await;
    }
}

impl AppState {
    fn default_runtime_stats(config: &FakerConfig) -> FakerStats {
        rustatio_core::RatioFaker::stats_from_config(config)
    }

    fn runtime_from_stats(stats: &FakerStats) -> PersistedRuntime {
        PersistedRuntime {
            uploaded: stats.uploaded,
            downloaded: stats.downloaded,
            ratio: stats.ratio,
            left: stats.left,
            torrent_completion: stats.torrent_completion,
            seeders: stats.seeders,
            leechers: stats.leechers,
            session_uploaded: stats.session_uploaded,
            session_downloaded: stats.session_downloaded,
            session_ratio: stats.session_ratio,
            elapsed_secs: stats.elapsed_time.as_secs(),
            current_upload_rate: stats.current_upload_rate,
            current_download_rate: stats.current_download_rate,
            average_upload_rate: stats.average_upload_rate,
            average_download_rate: stats.average_download_rate,
            upload_progress: stats.upload_progress,
            download_progress: stats.download_progress,
            ratio_progress: stats.ratio_progress,
            seed_time_progress: stats.seed_time_progress,
            effective_stop_at_ratio: stats.effective_stop_at_ratio,
            eta_ratio_secs: stats.eta_ratio.map(|value| value.as_secs()),
            eta_uploaded_secs: stats.eta_uploaded.map(|value| value.as_secs()),
            eta_seed_time_secs: stats.eta_seed_time.map(|value| value.as_secs()),
            eta_download_completion_secs: stats
                .eta_download_completion
                .map(|value| value.as_secs()),
            stop_condition_met: stats.stop_condition_met,
            is_idling: stats.is_idling,
            idling_reason: stats.idling_reason.clone(),
            announce_count: stats.announce_count,
        }
    }

    fn stats_from_runtime(
        runtime: &PersistedRuntime,
        state: FakerState,
        post_stop_action: rustatio_core::PostStopAction,
    ) -> FakerStats {
        FakerStats {
            uploaded: runtime.uploaded,
            downloaded: runtime.downloaded,
            ratio: runtime.ratio,
            left: runtime.left,
            torrent_completion: runtime.torrent_completion,
            seeders: runtime.seeders,
            leechers: runtime.leechers,
            state,
            is_idling: runtime.is_idling,
            idling_reason: runtime.idling_reason.clone(),
            session_uploaded: runtime.session_uploaded,
            session_downloaded: runtime.session_downloaded,
            session_ratio: runtime.session_ratio,
            elapsed_time: Duration::from_secs(runtime.elapsed_secs),
            current_upload_rate: runtime.current_upload_rate,
            current_download_rate: runtime.current_download_rate,
            average_upload_rate: runtime.average_upload_rate,
            average_download_rate: runtime.average_download_rate,
            upload_progress: runtime.upload_progress,
            download_progress: runtime.download_progress,
            ratio_progress: runtime.ratio_progress,
            seed_time_progress: runtime.seed_time_progress,
            effective_stop_at_ratio: runtime.effective_stop_at_ratio,
            eta_ratio: runtime.eta_ratio_secs.map(Duration::from_secs),
            eta_uploaded: runtime.eta_uploaded_secs.map(Duration::from_secs),
            eta_seed_time: runtime.eta_seed_time_secs.map(Duration::from_secs),
            eta_download_completion: runtime.eta_download_completion_secs.map(Duration::from_secs),
            upload_rate_history: Vec::new(),
            download_rate_history: Vec::new(),
            ratio_history: Vec::new(),
            history_timestamps: Vec::new(),
            last_announce: None,
            next_announce: None,
            announce_count: runtime.announce_count,
            stop_condition_met: runtime.stop_condition_met,
            post_stop_action,
        }
    }
}

impl EventBroadcaster for AppState {
    fn subscribe_logs(&self) -> broadcast::Receiver<LogEvent> {
        self.log_sender.subscribe()
    }

    fn subscribe_instance_events(&self) -> broadcast::Receiver<InstanceEvent> {
        self.instance_sender.subscribe()
    }

    fn emit_instance_event(&self, event: InstanceEvent) {
        let _ = self.instance_sender.send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustatio_core::{FakerConfig, FakerState, PostStopAction, PresetSettings, TorrentInfo};

    fn torrent() -> TorrentInfo {
        TorrentInfo {
            info_hash: [7u8; 20],
            announce: "https://tracker.test/announce".to_string(),
            announce_list: None,
            name: "sample".to_string(),
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

    async fn set_instance_state(state: &AppState, id: &str, faker_state: FakerState) {
        let instances = state.instances.read().await;
        let instance = instances.get(id);
        assert!(instance.is_some());
        if let Some(instance) = instance {
            let mut stats = instance.faker.stats_snapshot();
            stats.state = faker_state;
            instance.faker.restore_snapshot(stats).await;
        }
    }

    #[tokio::test]
    async fn apply_vpn_forwarded_port_updates_only_synced_instances() {
        let temp = tempfile::tempdir();
        assert!(temp.is_ok());
        let temp = temp.unwrap_or_else(|_| panic!("failed to create tempdir"));
        let state = AppState::new(&temp.path().to_string_lossy());

        let synced = FakerConfig { vpn_port_sync: true, port: 6881, ..FakerConfig::default() };

        let fixed = FakerConfig { vpn_port_sync: false, port: 60000, ..FakerConfig::default() };

        let created_synced = state.create_instance("synced", torrent(), synced).await;
        assert!(created_synced.is_ok());
        let created_fixed = state.create_instance("fixed", torrent(), fixed).await;
        assert!(created_fixed.is_ok());

        let applied = state.apply_vpn_forwarded_port(51413).await;
        assert!(applied.is_ok());
        assert_eq!(applied.unwrap_or_default(), 1);

        let instances = state.list_instances().await;
        let synced_inst = instances.iter().find(|inst| inst.id == "synced");
        assert!(synced_inst.is_some());
        assert_eq!(synced_inst.map(|inst| inst.config.port), Some(51413));

        let fixed_inst = instances.iter().find(|inst| inst.id == "fixed");
        assert!(fixed_inst.is_some());
        assert_eq!(fixed_inst.map(|inst| inst.config.port), Some(60000));
    }

    #[tokio::test]
    async fn effective_default_config_uses_forwarded_port_when_sync_enabled() {
        let temp = tempfile::tempdir();
        assert!(temp.is_ok());
        let temp = temp.unwrap_or_else(|_| panic!("failed to create tempdir"));
        let state = AppState::new(&temp.path().to_string_lossy());

        let config = FakerConfig { vpn_port_sync: true, port: 6881, ..FakerConfig::default() };

        let saved = state.set_default_config(Some(config)).await;
        assert!(saved.is_ok());
        state.set_current_forwarded_port(Some(45123));

        let effective = state.get_effective_default_config().await;
        assert!(effective.vpn_port_sync);
        assert_eq!(effective.port, 45123);
    }

    #[tokio::test]
    async fn update_instance_config_uses_forwarded_port_when_sync_enabled() {
        let temp = tempfile::tempdir();
        assert!(temp.is_ok());
        let temp = temp.unwrap_or_else(|_| panic!("failed to create tempdir"));
        let state = AppState::new(&temp.path().to_string_lossy());

        let config = FakerConfig { vpn_port_sync: true, port: 6881, ..FakerConfig::default() };

        let created = state.create_instance("synced", torrent(), config).await;
        assert!(created.is_ok());

        state.set_current_forwarded_port(Some(51413));

        let updated = state
            .update_instance_config(
                "synced",
                FakerConfig { vpn_port_sync: true, port: 6881, ..FakerConfig::default() },
            )
            .await;
        assert!(updated.is_ok());

        let instances = state.list_instances().await;
        let synced_inst = instances.iter().find(|inst| inst.id == "synced");
        assert!(synced_inst.is_some());
        assert_eq!(synced_inst.map(|inst| inst.config.port), Some(51413));
    }

    #[tokio::test]
    async fn update_instance_config_only_persists_vpn_sync_flag() {
        let temp = tempfile::tempdir();
        assert!(temp.is_ok());
        let temp = temp.unwrap_or_else(|_| panic!("failed to create tempdir"));
        let state = AppState::new(&temp.path().to_string_lossy());

        let created = state.create_instance("synced", torrent(), FakerConfig::default()).await;
        assert!(created.is_ok());

        state.set_current_forwarded_port(Some(51413));

        let updated = state
            .update_instance_config_only(
                "synced",
                FakerConfig { vpn_port_sync: true, port: 6881, ..FakerConfig::default() },
            )
            .await;
        assert!(updated.is_ok());

        let reloaded = state.persistence.load().await;
        let persisted = reloaded.instances.get("synced");
        assert!(persisted.is_some());
        assert_eq!(persisted.map(|inst| inst.config.vpn_port_sync), Some(true));
        assert_eq!(persisted.map(|inst| inst.config.port), Some(51413));
    }

    #[tokio::test]
    async fn apply_vpn_forwarded_port_updates_running_instance_port_immediately() {
        let temp = tempfile::tempdir();
        assert!(temp.is_ok());
        let temp = temp.unwrap_or_else(|_| panic!("failed to create tempdir"));
        let state = AppState::new(&temp.path().to_string_lossy());

        let created = state
            .create_instance(
                "synced",
                torrent(),
                FakerConfig { vpn_port_sync: true, port: 40000, ..FakerConfig::default() },
            )
            .await;
        assert!(created.is_ok());

        let started = state.start_instance("synced").await;
        assert!(started.is_ok());

        let applied = state.apply_vpn_forwarded_port(51413).await;
        assert!(applied.is_ok());
        assert_eq!(applied.unwrap_or_default(), 1);

        let instances = state.list_instances().await;
        let synced_inst = instances.iter().find(|inst| inst.id == "synced");
        assert!(synced_inst.is_some());
        assert_eq!(synced_inst.map(|inst| inst.config.port), Some(51413));
    }

    #[tokio::test]
    async fn desired_peer_port_rejects_mixed_forwarded_and_manual_active_ports() {
        let temp = tempfile::tempdir();
        assert!(temp.is_ok());
        let temp = temp.unwrap_or_else(|_| panic!("failed to create tempdir"));
        let state = AppState::new(&temp.path().to_string_lossy());

        let created_synced = state
            .create_instance(
                "synced",
                torrent(),
                FakerConfig { vpn_port_sync: true, port: 64429, ..FakerConfig::default() },
            )
            .await;
        assert!(created_synced.is_ok());

        let created_manual = state
            .create_instance(
                "manual",
                torrent(),
                FakerConfig { vpn_port_sync: false, port: 50000, ..FakerConfig::default() },
            )
            .await;
        assert!(created_manual.is_ok());

        set_instance_state(&state, "synced", FakerState::Running).await;
        set_instance_state(&state, "manual", FakerState::Running).await;
        state.set_current_forwarded_port(Some(53226));

        let desired = state.desired_peer_port_from_instances().await;
        assert!(desired.is_err());
        assert_eq!(
            desired.err(),
            Some("multiple active peer ports configured: 53226, 50000".to_string())
        );
    }

    #[tokio::test]
    async fn save_and_load_restores_paused_runtime_state() {
        let temp = tempfile::tempdir();
        assert!(temp.is_ok());
        let temp = match temp {
            Ok(value) => value,
            Err(e) => panic!("failed to create tempdir: {e}"),
        };
        let path = temp.path().to_string_lossy().to_string();
        let state = AppState::new(&path);

        let created = state.create_instance("paused", torrent(), FakerConfig::default()).await;
        assert!(created.is_ok());

        {
            let instances = state.instances.read().await;
            let instance = instances.get("paused");
            assert!(instance.is_some());
            if let Some(instance) = instance {
                let mut stats = instance.faker.stats_snapshot();
                stats.state = FakerState::Paused;
                stats.uploaded = 7_000;
                stats.downloaded = 3_000;
                stats.ratio = 6.8359;
                stats.session_uploaded = 2_000;
                stats.session_downloaded = 500;
                stats.session_ratio = 1.9531;
                stats.elapsed_time = Duration::from_secs(7200);
                stats.seed_time_progress = 50.0;
                stats.stop_condition_met = false;
                stats.post_stop_action = PostStopAction::Idle;
                instance.faker.restore_snapshot(stats).await;
            }
        }

        let saved = state.save_state().await;
        assert!(saved.is_ok());

        let restored = AppState::new(&path);
        let loaded = restored.load_saved_state().await;
        assert!(loaded.is_ok());
        let restored_count = loaded.unwrap_or(0);
        assert_eq!(restored_count, 1);

        let instances = restored.list_instances().await;
        assert_eq!(instances.len(), 1);
        let stats = &instances[0].stats;
        assert!(matches!(stats.state, FakerState::Paused));
        assert_eq!(stats.uploaded, 7_000);
        assert_eq!(stats.downloaded, 3_000);
        assert_eq!(stats.session_uploaded, 2_000);
        assert_eq!(stats.session_downloaded, 500);
        assert_eq!(stats.elapsed_time.as_secs(), 7200);
        assert_eq!(stats.seed_time_progress, 50.0);
    }

    #[tokio::test]
    async fn custom_and_default_presets_persist_to_state_file() {
        let temp = tempfile::tempdir();
        assert!(temp.is_ok());
        let temp = match temp {
            Ok(value) => value,
            Err(e) => panic!("failed to create tempdir: {e}"),
        };
        let state = AppState::new(&temp.path().to_string_lossy());

        let preset = CustomPreset {
            id: "custom-one".to_string(),
            name: "Custom One".to_string(),
            description: "Saved on server".to_string(),
            icon: "star".to_string(),
            custom: true,
            created_at: "2026-03-24T00:00:00Z".to_string(),
            settings: PresetSettings { upload_rate: Some(123.0), ..PresetSettings::default() },
        };

        let saved_preset = state.upsert_custom_preset(preset.clone()).await;
        assert!(saved_preset.is_ok());

        let default_saved = state
            .set_default_preset(Some(DefaultPreset {
                id: preset.id.clone(),
                name: preset.name.clone(),
                settings: preset.settings.clone(),
            }))
            .await;
        assert!(default_saved.is_ok());

        let persisted = state.persistence.load().await;
        assert_eq!(persisted.custom_presets.len(), 1);
        assert_eq!(persisted.custom_presets[0].id, "custom-one");
        assert_eq!(
            persisted.default_preset.as_ref().map(|item| item.id.as_str()),
            Some("custom-one")
        );
        assert_eq!(
            persisted.default_preset.as_ref().map(|item| item.name.as_str()),
            Some("Custom One")
        );
        assert_eq!(persisted.default_config.as_ref().map(|cfg| cfg.upload_rate), Some(123.0));
    }
}
