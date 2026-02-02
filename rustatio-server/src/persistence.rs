use rustatio_core::{FakerConfig, FakerState, TorrentInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use utoipa::ToSchema;

/// Source of an instance - where it was created from
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstanceSource {
    /// Created manually via UI/API
    #[default]
    Manual,
    /// Created automatically from watch folder
    WatchFolder,
}

/// Persisted state for a single faker instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedInstance {
    pub id: String,
    pub torrent: TorrentInfo,
    pub config: FakerConfig,
    pub cumulative_uploaded: u64,
    pub cumulative_downloaded: u64,
    pub state: FakerState,
    /// Timestamp when this instance was created
    pub created_at: u64,
    /// Timestamp of last update
    pub updated_at: u64,
    /// Source of this instance (manual or watch folder)
    #[serde(default)]
    pub source: InstanceSource,
}

/// Full application state that gets persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedState {
    pub instances: HashMap<String, PersistedInstance>,
    /// Default config for new instances (e.g., from watch folder)
    #[serde(default)]
    pub default_config: Option<FakerConfig>,
    /// Version for future migrations
    pub version: u32,
}

impl PersistedState {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            default_config: None,
            version: 1,
        }
    }
}

/// Persistence manager for saving/loading state
pub struct Persistence {
    state_file: String,
}

impl Persistence {
    pub fn new(data_dir: &str) -> Self {
        Self {
            state_file: format!("{}/state.json", data_dir),
        }
    }

    /// Load state from disk, returns default state if file doesn't exist
    pub async fn load(&self) -> PersistedState {
        let path = Path::new(&self.state_file);

        if !path.exists() {
            tracing::info!("No saved state found at {}, starting fresh", self.state_file);
            return PersistedState::new();
        }

        match fs::File::open(path).await {
            Ok(mut file) => {
                let mut contents = String::new();
                if let Err(e) = file.read_to_string(&mut contents).await {
                    tracing::error!("Failed to read state file: {}", e);
                    return PersistedState::new();
                }

                match serde_json::from_str(&contents) {
                    Ok(state) => {
                        tracing::info!("Loaded saved state from {}", self.state_file);
                        state
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse state file: {}", e);
                        // Backup corrupted file
                        let backup = format!("{}.corrupted", self.state_file);
                        let _ = fs::rename(path, &backup).await;
                        tracing::warn!("Backed up corrupted state to {}", backup);
                        PersistedState::new()
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to open state file: {}", e);
                PersistedState::new()
            }
        }
    }

    /// Save state to disk
    pub async fn save(&self, state: &PersistedState) -> Result<(), String> {
        // Ensure directory exists
        if let Some(parent) = Path::new(&self.state_file).parent() {
            if let Err(e) = fs::create_dir_all(parent).await {
                return Err(format!("Failed to create data directory: {}", e));
            }
        }

        // Serialize to JSON with pretty printing for debugging
        let json = serde_json::to_string_pretty(state).map_err(|e| format!("Failed to serialize state: {}", e))?;

        // Write to temp file first, then rename (atomic)
        let temp_file = format!("{}.tmp", self.state_file);

        let mut file = fs::File::create(&temp_file)
            .await
            .map_err(|e| format!("Failed to create temp file: {}", e))?;

        file.write_all(json.as_bytes())
            .await
            .map_err(|e| format!("Failed to write state: {}", e))?;

        file.sync_all()
            .await
            .map_err(|e| format!("Failed to sync state file: {}", e))?;

        // Atomic rename
        fs::rename(&temp_file, &self.state_file)
            .await
            .map_err(|e| format!("Failed to rename state file: {}", e))?;

        tracing::debug!("State saved to {}", self.state_file);
        Ok(())
    }
}

/// Get current timestamp in seconds since UNIX epoch
pub fn now_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
