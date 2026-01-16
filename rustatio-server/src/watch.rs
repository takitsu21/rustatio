//! Watch folder service for automatic torrent loading
//!
//! Watches a directory for .torrent files and automatically loads them as instances.
//! Optionally auto-starts faking with default configuration.

use crate::persistence::InstanceSource;
use crate::state::AppState;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rustatio_core::{FakerConfig, TorrentInfo};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Configuration for the watch folder service
#[derive(Debug, Clone)]
pub struct WatchConfig {
    /// Directory to watch for .torrent files
    pub watch_dir: PathBuf,
    /// Whether to auto-start faking when a torrent is loaded
    pub auto_start: bool,
    /// Whether the watch service is enabled
    pub enabled: bool,
}

/// Reason why watch folder is disabled
#[derive(Debug, Clone)]
pub enum WatchDisabledReason {
    /// Explicitly disabled via WATCH_ENABLED=false
    ExplicitlyDisabled,
    /// Watch directory does not exist and wasn't explicitly enabled
    DirectoryNotFound,
}

impl WatchConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> (Self, Option<WatchDisabledReason>) {
        let watch_dir = std::env::var("WATCH_DIR").unwrap_or_else(|_| "/torrents".to_string());
        let watch_path = PathBuf::from(&watch_dir);

        let auto_start = std::env::var("WATCH_AUTO_START")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(false);

        // Determine enabled status with reason tracking
        let (enabled, disabled_reason) = match std::env::var("WATCH_ENABLED") {
            Ok(val) => {
                let val_lower = val.to_lowercase();
                if val_lower == "false" || val == "0" {
                    // Explicitly disabled
                    (false, Some(WatchDisabledReason::ExplicitlyDisabled))
                } else if val_lower == "true" || val == "1" {
                    // Explicitly enabled
                    (true, None)
                } else if val.is_empty() {
                    // Empty string - use auto-detection
                    if watch_path.exists() && watch_path.is_dir() {
                        (true, None)
                    } else {
                        (false, Some(WatchDisabledReason::DirectoryNotFound))
                    }
                } else {
                    // Unknown value - treat as enabled (backwards compatible)
                    (true, None)
                }
            }
            Err(_) => {
                // Not set - auto-detect based on directory existence
                if watch_path.exists() && watch_path.is_dir() {
                    (true, None)
                } else {
                    (false, Some(WatchDisabledReason::DirectoryNotFound))
                }
            }
        };

        (
            Self {
                watch_dir: watch_path,
                auto_start,
                enabled,
            },
            disabled_reason,
        )
    }
}

/// Status of a torrent file in the watch folder
#[derive(Debug, Clone, Serialize)]
pub struct WatchedFile {
    pub filename: String,
    pub path: String,
    pub status: WatchedFileStatus,
    /// Info hash if successfully parsed (hex string)
    pub info_hash: Option<String>,
    /// Torrent name if successfully parsed
    pub name: Option<String>,
    /// File size in bytes
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WatchedFileStatus {
    /// File detected but not yet processed
    Pending,
    /// Successfully loaded as an instance
    Loaded,
    /// Duplicate - another instance with same info_hash exists
    Duplicate,
    /// Failed to parse as valid torrent
    Invalid,
}

/// Watch folder service status
#[derive(Debug, Clone, Serialize)]
pub struct WatchStatus {
    pub enabled: bool,
    pub watch_dir: String,
    pub auto_start: bool,
    pub file_count: usize,
    pub loaded_count: usize,
}

/// Watch folder service
pub struct WatchService {
    config: WatchConfig,
    state: AppState,
    /// Set of info_hashes that have been loaded (to detect duplicates)
    loaded_hashes: Arc<RwLock<HashSet<[u8; 20]>>>,
    /// Mapping from file path to info_hash (for handling file deletions)
    path_to_hash: Arc<RwLock<HashMap<PathBuf, [u8; 20]>>>,
    /// Shutdown signal sender
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl WatchService {
    pub fn new(config: WatchConfig, state: AppState) -> Self {
        Self {
            config,
            state,
            loaded_hashes: Arc::new(RwLock::new(HashSet::new())),
            path_to_hash: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: None,
        }
    }

    /// Get watch folder configuration
    pub fn config(&self) -> &WatchConfig {
        &self.config
    }

    /// Get the set of loaded info hashes
    pub fn loaded_hashes(&self) -> Arc<RwLock<HashSet<[u8; 20]>>> {
        self.loaded_hashes.clone()
    }

    /// Initialize loaded hashes from existing instances
    pub async fn init_from_state(&self) {
        let instances = self.state.list_instances().await;
        let mut hashes = self.loaded_hashes.write().await;
        for instance in instances {
            hashes.insert(instance.torrent.info_hash);
        }
        if !hashes.is_empty() {
            tracing::info!("Initialized watch service with {} existing torrents", hashes.len());
        }
    }

    /// Start the watch service
    pub async fn start(&mut self) -> Result<(), String> {
        if !self.config.enabled {
            tracing::info!("Watch folder service is disabled");
            return Ok(());
        }

        // Create watch directory if it doesn't exist
        if !self.config.watch_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&self.config.watch_dir) {
                return Err(format!("Failed to create watch directory: {}", e));
            }
            tracing::info!("Created watch directory: {:?}", self.config.watch_dir);
        }

        // Initialize loaded hashes from existing state
        self.init_from_state().await;

        // Scan existing files on startup
        self.scan_directory().await;

        // Start file watcher
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let watch_dir = self.config.watch_dir.clone();
        let auto_start = self.config.auto_start;
        let state = self.state.clone();
        let loaded_hashes = self.loaded_hashes.clone();
        let path_to_hash = self.path_to_hash.clone();

        tokio::spawn(async move {
            if let Err(e) = run_watcher(watch_dir, auto_start, state, loaded_hashes, path_to_hash, shutdown_rx).await {
                tracing::error!("Watch service error: {}", e);
            }
        });

        tracing::info!(
            "Watch folder service started: {:?} (auto_start={})",
            self.config.watch_dir,
            self.config.auto_start
        );

        Ok(())
    }

    /// Stop the watch service
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
            tracing::info!("Watch folder service stopped");
        }
    }

    /// Scan directory for existing .torrent files
    async fn scan_directory(&self) {
        let entries = match std::fs::read_dir(&self.config.watch_dir) {
            Ok(entries) => entries,
            Err(e) => {
                tracing::warn!("Failed to scan watch directory: {}", e);
                return;
            }
        };

        let mut count = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            if is_torrent_file(&path) {
                if let Err(e) = process_torrent_file(
                    &path,
                    self.config.auto_start,
                    &self.state,
                    &self.loaded_hashes,
                    &self.path_to_hash,
                )
                .await
                {
                    tracing::warn!("Failed to process {:?}: {}", path, e);
                } else {
                    count += 1;
                }
            }
        }

        if count > 0 {
            tracing::info!("Loaded {} torrent(s) from watch folder on startup", count);
        }
    }

    /// Get status of the watch service
    pub async fn get_status(&self) -> WatchStatus {
        let loaded_count = self.loaded_hashes.read().await.len();
        let file_count = std::fs::read_dir(&self.config.watch_dir)
            .map(|entries| {
                entries
                    .filter(|e| e.as_ref().map(|e| is_torrent_file(&e.path())).unwrap_or(false))
                    .count()
            })
            .unwrap_or(0);

        WatchStatus {
            enabled: self.config.enabled,
            watch_dir: self.config.watch_dir.to_string_lossy().to_string(),
            auto_start: self.config.auto_start,
            file_count,
            loaded_count,
        }
    }

    /// List all .torrent files in the watch folder with their status
    pub async fn list_files(&self) -> Vec<WatchedFile> {
        let mut files = Vec::new();
        let loaded_hashes = self.loaded_hashes.read().await;

        let entries = match std::fs::read_dir(&self.config.watch_dir) {
            Ok(entries) => entries,
            Err(_) => return files,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !is_torrent_file(&path) {
                continue;
            }

            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

            // Try to parse the torrent to get info
            let (status, info_hash, name) = match std::fs::read(&path) {
                Ok(data) => {
                    match TorrentInfo::from_bytes(&data) {
                        Ok(torrent) => {
                            let hash = torrent.info_hash;
                            let hash_hex = hex::encode(hash);
                            let torrent_name = torrent.name.clone();

                            let status = if loaded_hashes.contains(&hash) {
                                WatchedFileStatus::Loaded
                            } else {
                                // Check if any instance has this hash
                                WatchedFileStatus::Pending
                            };

                            (status, Some(hash_hex), Some(torrent_name))
                        }
                        Err(_) => (WatchedFileStatus::Invalid, None, None),
                    }
                }
                Err(_) => (WatchedFileStatus::Invalid, None, None),
            };

            files.push(WatchedFile {
                filename,
                path: path.to_string_lossy().to_string(),
                status,
                info_hash,
                name,
                size,
            });
        }

        // Sort by filename
        files.sort_by(|a, b| a.filename.cmp(&b.filename));
        files
    }

    /// Delete a torrent file from the watch folder and its corresponding instance
    pub async fn delete_file(&self, filename: &str) -> Result<(), String> {
        let path = self.config.watch_dir.join(filename);

        // Security: ensure the path is within watch_dir
        let canonical_watch = self
            .config
            .watch_dir
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize watch dir: {}", e))?;
        let canonical_file = path.canonicalize().map_err(|e| format!("File not found: {}", e))?;

        if !canonical_file.starts_with(&canonical_watch) {
            return Err("Invalid file path".to_string());
        }

        // Get the info_hash before deleting the file so we can delete the instance
        let info_hash = {
            let path_to_hash = self.path_to_hash.read().await;
            path_to_hash.get(&canonical_file).copied()
        };

        // Delete the file
        std::fs::remove_file(&canonical_file).map_err(|e| format!("Failed to delete file: {}", e))?;

        tracing::info!("Deleted torrent file: {}", filename);

        // Delete the corresponding instance if we have its info_hash
        if let Some(hash) = info_hash {
            // Remove from path_to_hash mapping
            self.path_to_hash.write().await.remove(&canonical_file);
            // Remove from loaded_hashes
            self.loaded_hashes.write().await.remove(&hash);
            // First, change the instance source to Manual (in case delete fails)
            if let Err(e) = self
                .state
                .update_instance_source_by_info_hash(&hash, InstanceSource::Manual)
                .await
            {
                tracing::warn!("Failed to update instance source: {}", e);
            }
            // Then delete the instance
            if let Err(e) = self.state.delete_instance_by_info_hash(&hash).await {
                tracing::warn!("Failed to delete instance for removed torrent: {}", e);
            }
        }

        Ok(())
    }
}

/// Check if a path is a .torrent file
fn is_torrent_file(path: &Path) -> bool {
    path.is_file() && path.extension().map(|e| e == "torrent").unwrap_or(false)
}

/// Process a torrent file - load it and optionally start faking
async fn process_torrent_file(
    path: &Path,
    auto_start: bool,
    state: &AppState,
    loaded_hashes: &Arc<RwLock<HashSet<[u8; 20]>>>,
    path_to_hash: &Arc<RwLock<HashMap<PathBuf, [u8; 20]>>>,
) -> Result<(), String> {
    // Read torrent file
    let data = std::fs::read(path).map_err(|e| format!("Failed to read torrent file: {}", e))?;

    // Parse torrent
    let torrent = TorrentInfo::from_bytes(&data).map_err(|e| format!("Failed to parse torrent: {}", e))?;

    let info_hash = torrent.info_hash;

    // Check for duplicates
    {
        let hashes = loaded_hashes.read().await;
        if hashes.contains(&info_hash) {
            tracing::warn!(
                "Skipping duplicate torrent '{}' (info_hash: {})",
                torrent.name,
                hex::encode(info_hash)
            );
            return Ok(());
        }
    }

    // Create instance with event emission for real-time sync
    let instance_id = state.next_instance_id().await;
    let config = FakerConfig::default();

    // Use create_instance_with_event so connected frontends get notified
    state
        .create_instance_with_event(&instance_id, torrent.clone(), config, auto_start)
        .await?;

    // Track as loaded
    loaded_hashes.write().await.insert(info_hash);

    // Record the path to info_hash mapping for deletion handling
    let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    path_to_hash.write().await.insert(canonical_path, info_hash);

    tracing::info!(
        "Loaded torrent '{}' from watch folder as instance {}",
        torrent.name,
        instance_id
    );

    // Auto-start if enabled
    if auto_start {
        if let Err(e) = state.start_instance(&instance_id).await {
            tracing::warn!("Failed to auto-start instance {}: {}", instance_id, e);
        } else {
            tracing::info!("Auto-started instance {}", instance_id);
        }
    }

    Ok(())
}

/// Run the file watcher in a background task
async fn run_watcher(
    watch_dir: PathBuf,
    auto_start: bool,
    state: AppState,
    loaded_hashes: Arc<RwLock<HashSet<[u8; 20]>>>,
    path_to_hash: Arc<RwLock<HashMap<PathBuf, [u8; 20]>>>,
    mut shutdown_rx: mpsc::Receiver<()>,
) -> Result<(), String> {
    let (tx, mut rx) = mpsc::channel(100);

    // Create watcher
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        },
        Config::default(),
    )
    .map_err(|e| format!("Failed to create watcher: {}", e))?;

    // Start watching
    watcher
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .map_err(|e| format!("Failed to watch directory: {}", e))?;

    tracing::debug!("File watcher started for {:?}", watch_dir);

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                tracing::debug!("File watcher received shutdown signal");
                break;
            }
            Some(event) = rx.recv() => {
                // Process create and modify events for .torrent files
                if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                    for path in event.paths {
                        if is_torrent_file(&path) {
                            // Small delay to ensure file is fully written
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                            if let Err(e) = process_torrent_file(
                                &path,
                                auto_start,
                                &state,
                                &loaded_hashes,
                                &path_to_hash,
                            ).await {
                                tracing::warn!("Failed to process {:?}: {}", path, e);
                            }
                        }
                    }
                }
                // Handle file removal events
                else if matches!(event.kind, EventKind::Remove(_)) {
                    for path in event.paths {
                        // Check if this was a torrent file we were tracking
                        // Note: We can't canonicalize the path because the file no longer exists
                        // So we need to search for it by matching the path or filename

                        // Try to find the info_hash for this path
                        let (info_hash, matched_path) = {
                            let mapping = path_to_hash.read().await;

                            // First try exact match with the path as given
                            if let Some(&hash) = mapping.get(&path) {
                                (Some(hash), Some(path.clone()))
                            } else {
                                // Try to find by matching filename in watch directory
                                // This handles the case where notify gives us a non-canonical path
                                // but we stored the canonical version
                                let filename = path.file_name();
                                if let Some(fname) = filename {
                                    let mut found = None;
                                    for (stored_path, &hash) in mapping.iter() {
                                        if stored_path.file_name() == Some(fname) {
                                            found = Some((hash, stored_path.clone()));
                                            break;
                                        }
                                    }
                                    match found {
                                        Some((hash, stored_path)) => (Some(hash), Some(stored_path)),
                                        None => (None, None),
                                    }
                                } else {
                                    (None, None)
                                }
                            }
                        };

                        if let Some(hash) = info_hash {
                            tracing::info!("Torrent file removed from watch folder: {:?}", path);

                            // Remove from path_to_hash mapping
                            if let Some(stored_path) = matched_path {
                                path_to_hash.write().await.remove(&stored_path);
                            }

                            // Remove from loaded_hashes
                            loaded_hashes.write().await.remove(&hash);

                            // First, change the instance source to Manual
                            // This ensures it can be deleted via UI if the delete below fails
                            if let Err(e) = state
                                .update_instance_source_by_info_hash(&hash, InstanceSource::Manual)
                                .await
                            {
                                tracing::warn!("Failed to update instance source: {}", e);
                            }

                            // Then delete the corresponding instance
                            if let Err(e) = state.delete_instance_by_info_hash(&hash).await {
                                tracing::warn!("Failed to delete instance for removed torrent: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
