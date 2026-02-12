use super::events::{EventBroadcaster, InstanceEvent, LogEvent};
use super::instance::{FakerInstance, InstanceInfo};
use super::persistence::{now_timestamp, InstanceSource, PersistedInstance, PersistedState, Persistence};
use rustatio_core::logger::set_instance_context_str;
use rustatio_core::{FakerConfig, FakerState, FakerStats, InstanceSummary, RatioFaker, TorrentInfo};
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

            match RatioFaker::new(persisted.torrent.clone(), faker_config) {
                Ok(faker) => {
                    let instance = FakerInstance {
                        faker: Arc::new(RwLock::new(faker)),
                        torrent: persisted.torrent.clone(),
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
            use super::lifecycle::InstanceLifecycle;
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
            let stats = instance.faker.read().await.get_stats().await;
            let mut config = instance.config.clone();
            config.completion_percent = stats.torrent_completion;

            persisted.instances.insert(
                id.clone(),
                PersistedInstance {
                    id: id.clone(),
                    torrent: instance.torrent.clone(),
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

    pub async fn next_instance_id(&self) -> String {
        nanoid::nanoid!(10)
    }

    pub async fn instance_exists(&self, id: &str) -> bool {
        self.instances.read().await.contains_key(id)
    }

    pub async fn update_instance_config(&self, id: &str, config: FakerConfig) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;

        let mut faker_config = config.clone();
        faker_config.initial_uploaded = instance.cumulative_uploaded;
        faker_config.initial_downloaded = instance.cumulative_downloaded;
        let existing_stats = instance.faker.read().await.get_stats().await;
        faker_config.completion_percent = existing_stats.torrent_completion;

        let faker = RatioFaker::new(instance.torrent.clone(), faker_config).map_err(|e| e.to_string())?;

        instance.faker = Arc::new(RwLock::new(faker));
        instance.config = config;

        Ok(())
    }

    pub async fn update_instance_config_only(&self, id: &str, config: FakerConfig) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;
        instance.config = config.clone();

        // Recreate the faker so stats reflect the new config (e.g. completion_percent)
        let faker =
            RatioFaker::new(instance.torrent.clone(), config).map_err(|e| format!("Failed to create faker: {}", e))?;
        instance.faker = Arc::new(RwLock::new(faker));

        drop(instances);
        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after config update: {}", e);
        }

        Ok(())
    }

    pub async fn create_instance(&self, id: &str, torrent: TorrentInfo, config: FakerConfig) -> Result<(), String> {
        self.create_instance_internal(id, torrent, config, InstanceSource::Manual)
            .await
    }

    pub async fn create_idle_instance(&self, id: &str, torrent: TorrentInfo) -> Result<(), String> {
        let config = FakerConfig::default();
        self.create_instance_internal(id, torrent.clone(), config, InstanceSource::Manual)
            .await?;

        self.emit_instance_event(InstanceEvent::Created {
            id: id.to_string(),
            torrent_name: torrent.name,
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
        self.create_instance_internal(id, torrent.clone(), config, InstanceSource::WatchFolder)
            .await?;

        self.emit_instance_event(InstanceEvent::Created {
            id: id.to_string(),
            torrent_name: torrent.name,
            info_hash: hex::encode(torrent.info_hash),
            auto_started,
        });

        Ok(())
    }

    async fn create_instance_internal(
        &self,
        id: &str,
        torrent: TorrentInfo,
        config: FakerConfig,
        source: InstanceSource,
    ) -> Result<(), String> {
        set_instance_context_str(Some(id));

        let torrent_info_hash = torrent.info_hash;

        let (
            cumulative_uploaded,
            cumulative_downloaded,
            created_at,
            existing_source,
            existing_tags,
            existing_completion,
        ) = {
            let instances = self.instances.read().await;
            if let Some(existing) = instances.get(id) {
                if existing.torrent_info_hash == torrent_info_hash {
                    let stats = existing.faker.read().await.get_stats().await;
                    (
                        existing.cumulative_uploaded,
                        existing.cumulative_downloaded,
                        existing.created_at,
                        Some(existing.source),
                        existing.tags.clone(),
                        Some(stats.torrent_completion),
                    )
                } else {
                    (0, 0, now_timestamp(), None, Vec::new(), None)
                }
            } else {
                (0, 0, now_timestamp(), None, Vec::new(), None)
            }
        };

        let final_source = existing_source.unwrap_or(source);

        let mut faker_config = config.clone();
        faker_config.initial_uploaded = cumulative_uploaded;
        faker_config.initial_downloaded = cumulative_downloaded;
        if let Some(completion) = existing_completion {
            faker_config.completion_percent = completion;
        }

        let faker = RatioFaker::new(torrent.clone(), faker_config).map_err(|e| e.to_string())?;

        let instance = FakerInstance {
            faker: Arc::new(RwLock::new(faker)),
            torrent: torrent.clone(),
            config,
            torrent_info_hash,
            cumulative_uploaded,
            cumulative_downloaded,
            created_at,
            source: final_source,
            tags: existing_tags,
        };

        self.instances.write().await.insert(id.to_string(), instance);

        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after creating instance: {}", e);
        }

        Ok(())
    }

    pub async fn get_stats(&self, id: &str) -> Result<FakerStats, String> {
        let faker = {
            let instances = self.instances.read().await;
            let instance = instances.get(id).ok_or("Instance not found")?;
            instance.faker.clone()
        };
        let stats = faker.read().await.get_stats().await;
        Ok(stats)
    }

    pub async fn get_instance_torrent(&self, id: &str) -> Result<TorrentInfo, String> {
        let instances = self.instances.read().await;
        let instance = instances.get(id).ok_or("Instance not found")?;
        Ok(instance.torrent.clone())
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
            let stats = instance.faker.read().await.get_stats().await;

            result.push(InstanceInfo {
                id: id.clone(),
                torrent: instance.torrent.clone(),
                config: instance.config.clone(),
                stats,
                created_at: instance.created_at,
                source: instance.source,
                tags: instance.tags.clone(),
            });
        }

        result
    }

    pub async fn get_instance_info_for_delete(&self, id: &str) -> Option<(InstanceSource, [u8; 20])> {
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

    pub async fn update_instance_source(&self, id: &str, source: InstanceSource) -> Result<(), String> {
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
        let id = match self.find_instance_by_info_hash(info_hash).await {
            Some(id) => id,
            None => return Ok(()),
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
            let stats = instance.faker.read().await.get_stats().await;

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
                name: instance.torrent.name.clone(),
                info_hash: hex::encode(instance.torrent_info_hash),
                state: state.to_string(),
                tags: instance.tags.clone(),
                total_size: instance.torrent.total_size,
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
        id: &str,
        torrent: TorrentInfo,
        config: FakerConfig,
        tags: Vec<String>,
        source: InstanceSource,
    ) -> Result<(), String> {
        self.create_instance_internal(id, torrent, config, source).await?;

        if !tags.is_empty() {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(id) {
                instance.tags = tags;
            }
        }

        Ok(())
    }

    pub async fn delete_instance_by_info_hash(&self, info_hash: &[u8; 20]) -> Result<(), String> {
        let id = match self.find_instance_by_info_hash(info_hash).await {
            Some(id) => id,
            None => return Ok(()),
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
            let stats = faker.get_stats().await;
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
