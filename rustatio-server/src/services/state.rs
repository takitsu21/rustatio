use super::events::{EventBroadcaster, InstanceEvent, LogEvent};
use super::instance::{FakerInstance, InstanceInfo};
use super::lifecycle::InstanceLifecycle;
use super::persistence::{
    now_timestamp, InstanceSource, PersistedInstance, PersistedState, Persistence,
};
use rustatio_core::logger::set_instance_context_str;
use rustatio_core::{
    FakerConfig, FakerState, FakerStats, InstanceSummary, RatioFaker, TorrentInfo, TorrentSummary,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub instances: Arc<RwLock<HashMap<String, FakerInstance>>>,
    pub log_sender: broadcast::Sender<LogEvent>,
    pub instance_sender: broadcast::Sender<InstanceEvent>,
    persistence: Arc<Persistence>,
    default_config: Arc<RwLock<Option<FakerConfig>>>,
    http_client: reqwest::Client,
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
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn get_default_config(&self) -> Option<FakerConfig> {
        self.default_config.read().await.clone()
    }

    pub async fn set_default_config(&self, config: Option<FakerConfig>) -> Result<(), String> {
        *self.default_config.write().await = config.clone();

        let existing = self.persistence.load().await;
        let mut updated = existing;
        updated.default_config = config;

        self.persistence.save(&updated).await
    }

    pub async fn load_saved_state(&self) -> Result<usize, String> {
        let saved = self.persistence.load().await;

        if let Some(config) = saved.default_config.clone() {
            *self.default_config.write().await = Some(config);
            tracing::info!("Restored default config from saved state");
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
            faker_config.initial_uploaded = persisted.cumulative_uploaded;
            faker_config.initial_downloaded = persisted.cumulative_downloaded;

            let summary = Arc::new(persisted.torrent.clone());
            let torrent = Arc::new(persisted.torrent.to_info());

            match RatioFaker::new(
                Arc::clone(&torrent),
                faker_config,
                Some(self.http_client.clone()),
            ) {
                Ok(faker) => {
                    let instance = FakerInstance {
                        faker: Arc::new(RwLock::new(faker)),
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

        let mut persisted = PersistedState {
            instances: HashMap::new(),
            default_config: self.default_config.read().await.clone(),
            version: 1,
        };

        for (id, instance) in instances.iter() {
            let stats = instance
                .faker
                .read()
                .await
                .stats_snapshot()
                .unwrap_or_else(|| RatioFaker::stats_from_config(&instance.config));
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

        let mut faker_config = config.clone();
        faker_config.initial_uploaded = instance.cumulative_uploaded;
        faker_config.initial_downloaded = instance.cumulative_downloaded;

        instance
            .faker
            .write()
            .await
            .update_config(faker_config, Some(self.http_client.clone()))
            .map_err(|e| e.to_string())?;
        instance.config = config;

        Ok(())
    }

    pub async fn update_instance_config_only(
        &self,
        id: &str,
        config: FakerConfig,
    ) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;
        instance.config = config.clone();

        instance
            .faker
            .write()
            .await
            .update_config(config, Some(self.http_client.clone()))
            .map_err(|e| format!("Failed to update faker config: {e}"))?;

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
                    let mut faker_config = config.clone();
                    faker_config.initial_uploaded = instance.cumulative_uploaded;
                    faker_config.initial_downloaded = instance.cumulative_downloaded;

                    let result = instance
                        .faker
                        .write()
                        .await
                        .update_config(faker_config, Some(self.http_client.clone()));
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

        (succeeded, failed)
    }

    pub async fn create_instance(
        &self,
        id: &str,
        torrent: TorrentInfo,
        config: FakerConfig,
    ) -> Result<(), String> {
        let context = InstanceBuildContext::new(id, torrent, config, InstanceSource::Manual);
        self.create_instance_internal(context).await
    }

    pub async fn create_idle_instance(&self, id: &str, torrent: TorrentInfo) -> Result<(), String> {
        let config = FakerConfig::default();
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
        let stats = faker.read().await.get_stats().await;
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
                let mut faker = instance.faker.write().await;
                let _ = faker.stop().await;
            }
        }

        let removed = self.instances.write().await.remove(id);

        if removed.is_some() {
            self.emit_instance_event(InstanceEvent::Deleted { id: id.to_string() });
        }

        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after deleting instance: {}", e);
        }

        Ok(())
    }

    pub async fn list_instances(&self) -> Vec<InstanceInfo> {
        let instances = self.instances.read().await;
        let mut result = Vec::new();

        for (id, instance) in instances.iter() {
            let stats = instance
                .faker
                .read()
                .await
                .stats_snapshot()
                .unwrap_or_else(|| RatioFaker::stats_from_config(&instance.config));

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
            let stats = instance
                .faker
                .read()
                .await
                .stats_snapshot()
                .unwrap_or_else(|| RatioFaker::stats_from_config(&instance.config));

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
                let stats = existing
                    .faker
                    .read()
                    .await
                    .stats_snapshot()
                    .unwrap_or_else(|| RatioFaker::stats_from_config(&existing.config));
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
        faker_config.initial_uploaded = existing.cumulative_uploaded;
        faker_config.initial_downloaded = existing.cumulative_downloaded;
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
            faker: Arc::new(RwLock::new(faker)),
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
                let mut faker = instance.faker.write().await;
                let _ = faker.stop().await;
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

        Ok(())
    }

    pub async fn shutdown_all(&self) {
        tracing::info!("Stopping all running faker instances...");

        let instances = self.instances.read().await;
        for (id, instance) in instances.iter() {
            let mut faker = instance.faker.write().await;
            let stats = faker
                .stats_snapshot()
                .unwrap_or_else(|| RatioFaker::stats_from_config(&FakerConfig::default()));
            if matches!(
                stats.state,
                FakerState::Starting | FakerState::Running | FakerState::Paused
            ) {
                if let Err(e) = faker.stop().await {
                    tracing::warn!("Failed to stop instance {}: {}", id, e);
                }
            }
        }
        drop(instances);

        tracing::info!("All faker instances stopped");
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
