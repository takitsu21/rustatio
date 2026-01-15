use crate::persistence::{now_timestamp, PersistedInstance, PersistedState, Persistence};
use rustatio_core::logger::set_instance_context;
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

/// Instance data with cumulative stats tracking
pub struct FakerInstance {
    pub faker: Arc<RwLock<RatioFaker>>,
    pub torrent: TorrentInfo,
    pub config: FakerConfig,
    pub torrent_info_hash: [u8; 20],
    pub cumulative_uploaded: u64,
    pub cumulative_downloaded: u64,
    pub created_at: u64,
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
    /// Counter for generating instance IDs
    next_id: Arc<RwLock<u32>>,
    /// Broadcast channel for log events (SSE)
    pub log_sender: broadcast::Sender<LogEvent>,
    /// Persistence manager
    persistence: Arc<Persistence>,
}

impl AppState {
    pub fn new(data_dir: &str) -> Self {
        let (log_sender, _) = broadcast::channel(256);
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            torrents: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
            log_sender,
            persistence: Arc::new(Persistence::new(data_dir)),
        }
    }

    /// Load saved state and restore instances
    pub async fn load_saved_state(&self) -> Result<usize, String> {
        let saved = self.persistence.load().await;

        // Update next_id
        *self.next_id.write().await = saved.next_id;

        let mut restored_count = 0;

        // Restore instances that were running
        for (id, persisted) in saved.instances {
            // Only restore instances that were running or paused
            match persisted.state {
                FakerState::Running | FakerState::Paused => {
                    tracing::info!("Restoring instance {} ({})", id, persisted.torrent.name);

                    // Create config with saved cumulative stats
                    let mut config = persisted.config.clone();
                    config.initial_uploaded = persisted.cumulative_uploaded;
                    config.initial_downloaded = persisted.cumulative_downloaded;

                    match RatioFaker::new(persisted.torrent.clone(), config.clone()) {
                        Ok(faker) => {
                            let instance = FakerInstance {
                                faker: Arc::new(RwLock::new(faker)),
                                torrent: persisted.torrent.clone(),
                                config: persisted.config,
                                torrent_info_hash: persisted.torrent.info_hash,
                                cumulative_uploaded: persisted.cumulative_uploaded,
                                cumulative_downloaded: persisted.cumulative_downloaded,
                                created_at: persisted.created_at,
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
                _ => {
                    tracing::debug!("Skipping instance {} with state {:?}", id, persisted.state);
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
        let next_id = *self.next_id.read().await;

        let mut persisted = PersistedState {
            instances: HashMap::new(),
            next_id,
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
                },
            );
        }

        self.persistence.save(&persisted).await
    }

    /// Subscribe to log events
    pub fn subscribe_logs(&self) -> broadcast::Receiver<LogEvent> {
        self.log_sender.subscribe()
    }

    /// Generate a new unique instance ID
    pub async fn next_instance_id(&self) -> String {
        let mut id = self.next_id.write().await;
        let current = *id;
        *id += 1;
        current.to_string()
    }

    /// Create a new faker instance
    pub async fn create_instance(&self, id: &str, torrent: TorrentInfo, config: FakerConfig) -> Result<(), String> {
        // Set instance context for logging
        set_instance_context(id.parse().ok());

        let torrent_info_hash = torrent.info_hash;

        // Check if instance exists and has same torrent - preserve cumulative stats
        let (cumulative_uploaded, cumulative_downloaded, created_at) = {
            let instances = self.instances.read().await;
            if let Some(existing) = instances.get(id) {
                if existing.torrent_info_hash == torrent_info_hash {
                    (
                        existing.cumulative_uploaded,
                        existing.cumulative_downloaded,
                        existing.created_at,
                    )
                } else {
                    (0, 0, now_timestamp())
                }
            } else {
                (0, 0, now_timestamp())
            }
        };

        // Apply cumulative stats to config
        let mut config = config;
        config.initial_uploaded = cumulative_uploaded;
        config.initial_downloaded = cumulative_downloaded;

        let faker = RatioFaker::new(torrent.clone(), config.clone()).map_err(|e| e.to_string())?;

        let instance = FakerInstance {
            faker: Arc::new(RwLock::new(faker)),
            torrent: torrent.clone(),
            config,
            torrent_info_hash,
            cumulative_uploaded,
            cumulative_downloaded,
            created_at,
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
        set_instance_context(id.parse().ok());

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
                    set_instance_context(id.parse().ok());
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
        set_instance_context(id.parse().ok());

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
        set_instance_context(id.parse().ok());

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
        set_instance_context(id.parse().ok());

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
        set_instance_context(id.parse().ok());

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
        set_instance_context(id.parse().ok());

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
    pub async fn delete_instance(&self, id: &str) -> Result<(), String> {
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
        self.instances.write().await.remove(id);

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
            });
        }

        result
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
