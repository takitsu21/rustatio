use crate::persistence::{now_timestamp, InstanceSource, PersistedInstance, PersistedState, Persistence};
use rustatio_core::logger::set_instance_context_str;
use rustatio_core::{FakerConfig, FakerState, FakerStats, RatioFaker, TorrentInfo};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::task::JoinHandle;

/// Log event sent to UI via SSE
#[derive(Clone, Debug, Serialize)]
pub struct LogEvent {
    pub timestamp: u64,
    pub level: String,
    pub message: String,
}

impl LogEvent {
    pub fn new(level: &str, message: String) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            timestamp,
            level: level.to_string(),
            message,
        }
    }
}

/// Instance event sent to UI via SSE for real-time sync
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstanceEvent {
    /// A new instance was created (e.g., from watch folder)
    Created {
        id: String,
        torrent_name: String,
        info_hash: String,
        auto_started: bool,
    },
    /// An instance was deleted
    Deleted { id: String },
}

/// Instance data with cumulative stats tracking
pub struct FakerInstance {
    pub faker: Arc<RwLock<RatioFaker>>,
    pub torrent: TorrentInfo,
    pub config: FakerConfig,
    pub torrent_info_hash: [u8; 20],
    pub cumulative_uploaded: u64,
    pub cumulative_downloaded: u64,
    pub created_at: u64,
    /// Source of this instance (manual or watch folder)
    pub source: InstanceSource,
    /// Background task handle (if running)
    task_handle: Option<JoinHandle<()>>,
    /// Shutdown signal sender for background task
    shutdown_tx: Option<mpsc::Sender<()>>,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Active faker instances
    pub instances: Arc<RwLock<HashMap<String, FakerInstance>>>,
    /// Loaded torrents (not yet started)
    pub torrents: Arc<RwLock<HashMap<String, TorrentInfo>>>,
    /// Broadcast channel for log events (SSE)
    pub log_sender: broadcast::Sender<LogEvent>,
    /// Broadcast channel for instance events (SSE)
    pub instance_sender: broadcast::Sender<InstanceEvent>,
    /// Persistence manager
    persistence: Arc<Persistence>,
    /// Default config for new instances (e.g., from watch folder)
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
    
    /// Get the default config for new instances
    pub async fn get_default_config(&self) -> Option<FakerConfig> {
        self.default_config.read().await.clone()
    }
    
    /// Set the default config for new instances
    pub async fn set_default_config(&self, config: Option<FakerConfig>) -> Result<(), String> {
        *self.default_config.write().await = config.clone();
        
        // Also persist it
        let mut state = PersistedState::new();
        state.default_config = config;
        
        // Load existing state and update just the default_config
        let existing = self.persistence.load().await;
        let mut updated = existing;
        updated.default_config = state.default_config;
        
        self.persistence.save(&updated).await
    }

    /// Load saved state and restore instances
    pub async fn load_saved_state(&self) -> Result<usize, String> {
        let saved = self.persistence.load().await;
        
        // Restore default config if present
        if let Some(config) = saved.default_config.clone() {
            *self.default_config.write().await = Some(config);
            tracing::info!("Restored default config from saved state");
        }

        let mut restored_count = 0;

        // Restore all instances (including Idle ones so they persist across refreshes)
        for (id, persisted) in saved.instances {
            tracing::info!(
                "Restoring instance {} ({}) - state: {:?}",
                id,
                persisted.torrent.name,
                persisted.state
            );

            // Create config with saved cumulative stats for RatioFaker
            // But store the original persisted.config in FakerInstance
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

                    // Auto-start if it was running
                    if matches!(persisted.state, FakerState::Running) {
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

    /// Save current state to disk
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

    /// Subscribe to log events
    pub fn subscribe_logs(&self) -> broadcast::Receiver<LogEvent> {
        self.log_sender.subscribe()
    }

    /// Subscribe to instance events (for real-time sync with frontend)
    pub fn subscribe_instance_events(&self) -> broadcast::Receiver<InstanceEvent> {
        self.instance_sender.subscribe()
    }

    /// Emit an instance event to all subscribers
    pub fn emit_instance_event(&self, event: InstanceEvent) {
        // Ignore send errors (no subscribers is fine)
        let _ = self.instance_sender.send(event);
    }

    /// Generate a new unique instance ID using nanoid
    pub async fn next_instance_id(&self) -> String {
        nanoid::nanoid!(10) // 10 chars is short but collision-resistant enough
    }

    /// Check if an instance exists
    pub async fn instance_exists(&self, id: &str) -> bool {
        self.instances.read().await.contains_key(id)
    }

    /// Update an existing instance's config (used when starting an existing instance with new config)
    pub async fn update_instance_config(&self, id: &str, config: FakerConfig) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;

        // Create a separate config for RatioFaker with cumulative stats as initial values
        let mut faker_config = config.clone();
        faker_config.initial_uploaded = instance.cumulative_uploaded;
        faker_config.initial_downloaded = instance.cumulative_downloaded;

        let faker = RatioFaker::new(instance.torrent.clone(), faker_config).map_err(|e| e.to_string())?;

        instance.faker = Arc::new(RwLock::new(faker));
        instance.config = config.clone(); // Store original user config (not modified)

        Ok(())
    }

    /// Update only the config for an instance (without recreating the faker)
    /// Used to persist form changes before the faker is started
    pub async fn update_instance_config_only(&self, id: &str, config: FakerConfig) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;

        // Just update the stored config, don't recreate the faker
        instance.config = config;

        // Save state to persist the config change
        drop(instances); // Release lock before calling save_state
        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after config update: {}", e);
        }

        Ok(())
    }

    /// Create a new faker instance (manual creation via API)
    pub async fn create_instance(&self, id: &str, torrent: TorrentInfo, config: FakerConfig) -> Result<(), String> {
        self.create_instance_internal(id, torrent, config, InstanceSource::Manual)
            .await
    }

    /// Create a new idle faker instance (torrent loaded but not started)
    /// Used when user loads a torrent via UI - creates server-side instance so it persists on refresh
    pub async fn create_idle_instance(&self, id: &str, torrent: TorrentInfo) -> Result<(), String> {
        // Use default config for idle instance
        let config = FakerConfig::default();
        self.create_instance_internal(id, torrent.clone(), config, InstanceSource::Manual)
            .await?;

        // Emit event for real-time sync
        self.emit_instance_event(InstanceEvent::Created {
            id: id.to_string(),
            torrent_name: torrent.name,
            info_hash: hex::encode(torrent.info_hash),
            auto_started: false,
        });

        Ok(())
    }

    /// Create a new faker instance and emit an event for real-time sync
    /// Used by watch folder to notify connected frontends
    pub async fn create_instance_with_event(
        &self,
        id: &str,
        torrent: TorrentInfo,
        config: FakerConfig,
        auto_started: bool,
    ) -> Result<(), String> {
        self.create_instance_internal(id, torrent.clone(), config, InstanceSource::WatchFolder)
            .await?;

        // Emit event for real-time sync
        self.emit_instance_event(InstanceEvent::Created {
            id: id.to_string(),
            torrent_name: torrent.name,
            info_hash: hex::encode(torrent.info_hash),
            auto_started,
        });

        Ok(())
    }

    /// Internal implementation for creating instances
    async fn create_instance_internal(
        &self,
        id: &str,
        torrent: TorrentInfo,
        config: FakerConfig,
        source: InstanceSource,
    ) -> Result<(), String> {
        // Set instance context for logging
        set_instance_context_str(Some(id));

        let torrent_info_hash = torrent.info_hash;

        // Check if instance exists and has same torrent - preserve cumulative stats and source
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

        // Preserve existing source if instance already exists, otherwise use provided source
        let final_source = existing_source.unwrap_or(source);

        // Create a separate config for RatioFaker with cumulative stats as initial values
        // This ensures the faker starts from cumulative totals, but we preserve the
        // original user config for display in the frontend
        let mut faker_config = config.clone();
        faker_config.initial_uploaded = cumulative_uploaded;
        faker_config.initial_downloaded = cumulative_downloaded;

        let faker = RatioFaker::new(torrent.clone(), faker_config).map_err(|e| e.to_string())?;

        let instance = FakerInstance {
            faker: Arc::new(RwLock::new(faker)),
            torrent: torrent.clone(),
            config: config.clone(), // Store original user config (not modified)
            torrent_info_hash,
            cumulative_uploaded,
            cumulative_downloaded,
            created_at,
            source: final_source,
            task_handle: None,
            shutdown_tx: None,
        };

        self.instances.write().await.insert(id.to_string(), instance);

        // Save state after creating instance
        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after creating instance: {}", e);
        }

        Ok(())
    }

    /// Start a faker instance
    pub async fn start_instance(&self, id: &str) -> Result<(), String> {
        // Set instance context for logging
        set_instance_context_str(Some(id));

        let faker_arc = {
            let mut instances = self.instances.write().await;
            let instance = instances.get_mut(id).ok_or("Instance not found")?;

            // Stop existing background task if any
            if let Some(tx) = instance.shutdown_tx.take() {
                let _ = tx.send(()).await;
            }
            if let Some(handle) = instance.task_handle.take() {
                handle.abort();
            }

            instance.faker.clone()
        };

        // Start the faker (sends "started" announce)
        faker_arc.write().await.start().await.map_err(|e| e.to_string())?;

        // Spawn background update task
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        let id_clone = id.to_string();
        let faker_clone = faker_arc.clone();
        let instances_clone = self.instances.clone();
        let persistence_self = self.clone();

        let task_handle = tokio::spawn(async move {
            Self::background_update_loop(id_clone, faker_clone, instances_clone, persistence_self, shutdown_rx).await;
        });

        // Store task handle and shutdown sender
        {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(id) {
                instance.task_handle = Some(task_handle);
                instance.shutdown_tx = Some(shutdown_tx);
            }
        }

        // Save state after starting
        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after starting instance: {}", e);
        }

        Ok(())
    }

    /// Background update loop that runs independently of client polling
    async fn background_update_loop(
        id: String,
        faker: Arc<RwLock<RatioFaker>>,
        instances: Arc<RwLock<HashMap<String, FakerInstance>>>,
        state: AppState,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) {
        let update_interval = Duration::from_secs(5);
        let save_interval = Duration::from_secs(30);
        let mut last_save = std::time::Instant::now();

        tracing::info!("Background update loop started for instance {}", id);

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    tracing::info!("Background update loop received shutdown signal for instance {}", id);
                    break;
                }
                _ = tokio::time::sleep(update_interval) => {
                    // Check if instance still exists and is running
                    let should_continue = {
                        let instances_guard = instances.read().await;
                        if let Some(instance) = instances_guard.get(&id) {
                            let stats = instance.faker.read().await.get_stats().await;
                            matches!(stats.state, FakerState::Running)
                        } else {
                            false
                        }
                    };

                    if !should_continue {
                        tracing::info!("Instance {} no longer running, stopping background loop", id);
                        break;
                    }

                    // Update the faker (calculates stats, may trigger tracker announce)
                    set_instance_context_str(Some(&id));
                    if let Err(e) = faker.write().await.update().await {
                        tracing::warn!("Background update failed for instance {}: {}", id, e);
                    }

                    // Periodically save state
                    if last_save.elapsed() >= save_interval {
                        if let Err(e) = state.save_state().await {
                            tracing::warn!("Failed to save state in background loop: {}", e);
                        }
                        last_save = std::time::Instant::now();
                    }
                }
            }
        }

        tracing::info!("Background update loop stopped for instance {}", id);
    }

    /// Stop a faker instance
    pub async fn stop_instance(&self, id: &str) -> Result<FakerStats, String> {
        // Set instance context for logging
        set_instance_context_str(Some(id));

        let (faker_arc, shutdown_tx, task_handle) = {
            let mut instances = self.instances.write().await;
            let instance = instances.get_mut(id).ok_or("Instance not found")?;
            (
                instance.faker.clone(),
                instance.shutdown_tx.take(),
                instance.task_handle.take(),
            )
        };

        // Signal background task to stop
        if let Some(tx) = shutdown_tx {
            let _ = tx.send(()).await;
        }
        // Wait for task to finish (with timeout)
        if let Some(handle) = task_handle {
            let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
        }

        // Get final stats before stopping
        let stats = faker_arc.read().await.get_stats().await;

        // Stop the faker (sends "stopped" announce)
        faker_arc.write().await.stop().await.map_err(|e| e.to_string())?;

        // Update cumulative stats
        {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(id) {
                instance.cumulative_uploaded = stats.uploaded;
                instance.cumulative_downloaded = stats.downloaded;
            }
        }

        // Save state after stopping
        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after stopping instance: {}", e);
        }

        Ok(stats)
    }

    /// Pause a faker instance
    pub async fn pause_instance(&self, id: &str) -> Result<(), String> {
        // Set instance context for logging
        set_instance_context_str(Some(id));

        let (faker_arc, shutdown_tx, task_handle) = {
            let mut instances = self.instances.write().await;
            let instance = instances.get_mut(id).ok_or("Instance not found")?;
            (
                instance.faker.clone(),
                instance.shutdown_tx.take(),
                instance.task_handle.take(),
            )
        };

        // Signal background task to stop
        if let Some(tx) = shutdown_tx {
            let _ = tx.send(()).await;
        }
        // Wait for task to finish (with timeout)
        if let Some(handle) = task_handle {
            let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
        }

        // Pause the faker
        faker_arc.write().await.pause().await.map_err(|e| e.to_string())?;

        // Save state after pausing
        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after pausing instance: {}", e);
        }

        Ok(())
    }

    /// Resume a faker instance
    pub async fn resume_instance(&self, id: &str) -> Result<(), String> {
        // Set instance context for logging
        set_instance_context_str(Some(id));

        let faker_arc = {
            let mut instances = self.instances.write().await;
            let instance = instances.get_mut(id).ok_or("Instance not found")?;

            // Stop existing background task if any (shouldn't have one when paused, but be safe)
            if let Some(tx) = instance.shutdown_tx.take() {
                let _ = tx.send(()).await;
            }
            if let Some(handle) = instance.task_handle.take() {
                handle.abort();
            }

            instance.faker.clone()
        };

        // Resume the faker
        faker_arc.write().await.resume().await.map_err(|e| e.to_string())?;

        // Spawn background update task
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        let id_clone = id.to_string();
        let faker_clone = faker_arc.clone();
        let instances_clone = self.instances.clone();
        let persistence_self = self.clone();

        let task_handle = tokio::spawn(async move {
            Self::background_update_loop(id_clone, faker_clone, instances_clone, persistence_self, shutdown_rx).await;
        });

        // Store task handle and shutdown sender
        {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(id) {
                instance.task_handle = Some(task_handle);
                instance.shutdown_tx = Some(shutdown_tx);
            }
        }

        // Save state after resuming
        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after resuming instance: {}", e);
        }

        Ok(())
    }

    /// Update faker (send tracker announce)
    pub async fn update_instance(&self, id: &str) -> Result<FakerStats, String> {
        // Set instance context for logging
        set_instance_context_str(Some(id));

        let faker_arc = {
            let instances = self.instances.read().await;
            let instance = instances.get(id).ok_or("Instance not found")?;
            instance.faker.clone()
        };

        faker_arc.write().await.update().await.map_err(|e| e.to_string())?;
        let stats = faker_arc.read().await.get_stats().await;
        Ok(stats)
    }

    /// Update stats only (no tracker announce)
    pub async fn update_stats_only(&self, id: &str) -> Result<FakerStats, String> {
        // Set instance context for logging
        set_instance_context_str(Some(id));

        let faker_arc = {
            let instances = self.instances.read().await;
            let instance = instances.get(id).ok_or("Instance not found")?;
            instance.faker.clone()
        };

        faker_arc
            .write()
            .await
            .update_stats_only()
            .await
            .map_err(|e| e.to_string())?;
        let stats = faker_arc.read().await.get_stats().await;
        Ok(stats)
    }

    /// Get stats for an instance
    pub async fn get_stats(&self, id: &str) -> Result<FakerStats, String> {
        let faker_arc = {
            let instances = self.instances.read().await;
            let instance = instances.get(id).ok_or("Instance not found")?;
            instance.faker.clone()
        };
        let stats = faker_arc.read().await.get_stats().await;
        Ok(stats)
    }

    /// Delete an instance (idempotent - returns Ok even if not found)
    /// Note: Watch folder instances cannot be deleted via API unless force=true
    /// Use force=true for orphaned watch folder instances (file no longer exists)
    pub async fn delete_instance(&self, id: &str, force: bool) -> Result<(), String> {
        // Check if instance exists and if it's from watch folder (unless force=true)
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

        // Stop background task if running
        let (shutdown_tx, task_handle) = {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(id) {
                (instance.shutdown_tx.take(), instance.task_handle.take())
            } else {
                (None, None)
            }
        };

        // Signal background task to stop
        if let Some(tx) = shutdown_tx {
            let _ = tx.send(()).await;
        }
        // Wait for task to finish (with timeout)
        if let Some(handle) = task_handle {
            let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
        }

        // Remove instance
        let removed = self.instances.write().await.remove(id);

        // Emit event if instance was actually removed
        if removed.is_some() {
            self.emit_instance_event(InstanceEvent::Deleted { id: id.to_string() });
        }

        // Save state after deleting
        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after deleting instance: {}", e);
        }

        Ok(())
    }

    /// Store a loaded torrent
    pub async fn store_torrent(&self, id: &str, torrent: TorrentInfo) {
        self.torrents.write().await.insert(id.to_string(), torrent);
    }

    /// Get a stored torrent
    #[allow(dead_code)]
    pub async fn get_torrent(&self, id: &str) -> Option<TorrentInfo> {
        self.torrents.read().await.get(id).cloned()
    }

    /// List all instances with their current stats
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

    /// Find instance ID by info_hash
    pub async fn find_instance_by_info_hash(&self, info_hash: &[u8; 20]) -> Option<String> {
        let instances = self.instances.read().await;
        for (id, instance) in instances.iter() {
            if &instance.torrent_info_hash == info_hash {
                return Some(id.clone());
            }
        }
        None
    }

    /// Update an instance's source
    pub async fn update_instance_source(&self, id: &str, source: InstanceSource) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;
        instance.source = source;
        drop(instances);

        // Save state after updating source
        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after updating instance source: {}", e);
        }

        Ok(())
    }

    /// Update an instance's source by info_hash
    pub async fn update_instance_source_by_info_hash(
        &self,
        info_hash: &[u8; 20],
        source: InstanceSource,
    ) -> Result<(), String> {
        let id = match self.find_instance_by_info_hash(info_hash).await {
            Some(id) => id,
            None => return Ok(()), // No instance found, nothing to update
        };
        self.update_instance_source(&id, source).await
    }

    /// Delete an instance by info_hash (internal use - bypasses source check)
    /// Used when torrent file is removed from watch folder
    pub async fn delete_instance_by_info_hash(&self, info_hash: &[u8; 20]) -> Result<(), String> {
        // Find the instance ID
        let id = match self.find_instance_by_info_hash(info_hash).await {
            Some(id) => id,
            None => return Ok(()), // No instance found, nothing to delete
        };

        // Stop background task if running
        let (shutdown_tx, task_handle) = {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(&id) {
                (instance.shutdown_tx.take(), instance.task_handle.take())
            } else {
                (None, None)
            }
        };

        // Signal background task to stop
        if let Some(tx) = shutdown_tx {
            let _ = tx.send(()).await;
        }
        // Wait for task to finish (with timeout)
        if let Some(handle) = task_handle {
            let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
        }

        // Remove instance
        let removed = self.instances.write().await.remove(&id);

        // Emit event if instance was actually removed
        if removed.is_some() {
            tracing::info!("Deleted instance {} (torrent file removed from watch folder)", id);
            self.emit_instance_event(InstanceEvent::Deleted { id: id.clone() });
        }

        // Save state after deleting
        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after deleting instance: {}", e);
        }

        Ok(())
    }
}

/// Information about an instance for the list endpoint
#[derive(Debug, Clone, Serialize)]
pub struct InstanceInfo {
    pub id: String,
    pub torrent: TorrentInfo,
    pub config: FakerConfig,
    pub stats: FakerStats,
    pub created_at: u64,
    pub source: InstanceSource,
}

impl AppState {
    /// Stop all background tasks (call on server shutdown)
    pub async fn shutdown_all(&self) {
        tracing::info!("Shutting down all background tasks...");

        let mut instances = self.instances.write().await;
        let mut handles = Vec::new();

        for (id, instance) in instances.iter_mut() {
            // Signal background task to stop
            if let Some(tx) = instance.shutdown_tx.take() {
                let _ = tx.send(()).await;
            }
            // Collect handles for waiting
            if let Some(handle) = instance.task_handle.take() {
                handles.push((id.clone(), handle));
            }
        }
        drop(instances);

        // Wait for all tasks to finish (with timeout)
        for (id, handle) in handles {
            match tokio::time::timeout(Duration::from_secs(5), handle).await {
                Ok(_) => tracing::debug!("Background task for instance {} stopped", id),
                Err(_) => tracing::warn!("Timeout waiting for background task {} to stop", id),
            }
        }

        tracing::info!("All background tasks stopped");
    }
}
