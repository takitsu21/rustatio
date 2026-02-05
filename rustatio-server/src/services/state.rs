use super::events::{EventBroadcaster, InstanceEvent, LogEvent};
use super::instance::{FakerInstance, InstanceInfo};
use super::persistence::{now_timestamp, InstanceSource, PersistedInstance, PersistedState, Persistence};
use rustatio_core::logger::set_instance_context_str;
use rustatio_core::{FakerConfig, FakerState, FakerStats, RatioFaker, TorrentInfo};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub instances: Arc<RwLock<HashMap<String, FakerInstance>>>,
    pub torrents: Arc<RwLock<HashMap<String, TorrentInfo>>>,
    pub log_sender: broadcast::Sender<LogEvent>,
    pub instance_sender: broadcast::Sender<InstanceEvent>,
    persistence: Arc<Persistence>,
    default_config: Arc<RwLock<Option<FakerConfig>>>,
}

impl AppState {
    pub fn new(data_dir: &str) -> Self {
        let (log_sender, _) = broadcast::channel(256);
        let (instance_sender, _) = broadcast::channel(64);
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            torrents: Arc::new(RwLock::new(HashMap::new())),
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

        for (id, persisted) in saved.instances {
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
                        config: persisted.config,
                        torrent_info_hash: persisted.torrent.info_hash,
                        cumulative_uploaded: persisted.cumulative_uploaded,
                        cumulative_downloaded: persisted.cumulative_downloaded,
                        created_at: persisted.created_at,
                        source: persisted.source,
                        task_handle: None,
                        shutdown_tx: None,
                    };

                    self.instances.write().await.insert(id.clone(), instance);

                    if matches!(persisted.state, FakerState::Running) {
                        // Import the trait to call start_instance
                        use super::lifecycle::InstanceLifecycle;
                        if let Err(e) = self.start_instance(&id).await {
                            tracing::warn!("Failed to auto-start instance {}: {}", id, e);
                        }
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

            persisted.instances.insert(
                id.clone(),
                PersistedInstance {
                    id: id.clone(),
                    torrent: instance.torrent.clone(),
                    config: instance.config.clone(),
                    cumulative_uploaded: stats.uploaded,
                    cumulative_downloaded: stats.downloaded,
                    state: stats.state,
                    created_at: instance.created_at,
                    updated_at: now_timestamp(),
                    source: instance.source,
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

        let faker = RatioFaker::new(instance.torrent.clone(), faker_config).map_err(|e| e.to_string())?;

        instance.faker = Arc::new(RwLock::new(faker));
        instance.config = config;

        Ok(())
    }

    pub async fn update_instance_config_only(&self, id: &str, config: FakerConfig) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;
        instance.config = config;

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

        let (cumulative_uploaded, cumulative_downloaded, created_at, existing_source) = {
            let instances = self.instances.read().await;
            if let Some(existing) = instances.get(id) {
                if existing.torrent_info_hash == torrent_info_hash {
                    (
                        existing.cumulative_uploaded,
                        existing.cumulative_downloaded,
                        existing.created_at,
                        Some(existing.source),
                    )
                } else {
                    (0, 0, now_timestamp(), None)
                }
            } else {
                (0, 0, now_timestamp(), None)
            }
        };

        let final_source = existing_source.unwrap_or(source);

        let mut faker_config = config.clone();
        faker_config.initial_uploaded = cumulative_uploaded;
        faker_config.initial_downloaded = cumulative_downloaded;

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
            task_handle: None,
            shutdown_tx: None,
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

        let (shutdown_tx, task_handle) = {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(id) {
                (instance.shutdown_tx.take(), instance.task_handle.take())
            } else {
                (None, None)
            }
        };

        if let Some(tx) = shutdown_tx {
            let _ = tx.send(()).await;
        }
        if let Some(handle) = task_handle {
            let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
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

    pub async fn store_torrent(&self, id: &str, torrent: TorrentInfo) {
        self.torrents.write().await.insert(id.to_string(), torrent);
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

    pub async fn delete_instance_by_info_hash(&self, info_hash: &[u8; 20]) -> Result<(), String> {
        let id = match self.find_instance_by_info_hash(info_hash).await {
            Some(id) => id,
            None => return Ok(()),
        };

        let (shutdown_tx, task_handle) = {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(&id) {
                (instance.shutdown_tx.take(), instance.task_handle.take())
            } else {
                (None, None)
            }
        };

        if let Some(tx) = shutdown_tx {
            let _ = tx.send(()).await;
        }
        if let Some(handle) = task_handle {
            let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
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
        tracing::info!("Shutting down all background tasks...");

        let mut instances = self.instances.write().await;
        let mut handles = Vec::new();

        for (id, instance) in instances.iter_mut() {
            if let Some(tx) = instance.shutdown_tx.take() {
                let _ = tx.send(()).await;
            }
            if let Some(handle) = instance.task_handle.take() {
                handles.push((id.clone(), handle));
            }
        }
        drop(instances);

        for (id, handle) in handles {
            match tokio::time::timeout(Duration::from_secs(5), handle).await {
                Ok(_) => tracing::debug!("Background task for instance {} stopped", id),
                Err(_) => tracing::warn!("Timeout waiting for background task {} to stop", id),
            }
        }

        tracing::info!("All background tasks stopped");
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
