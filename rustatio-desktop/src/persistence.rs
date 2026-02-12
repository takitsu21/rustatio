use rustatio_core::{FakerConfig, FakerState, TorrentInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedInstance {
    pub id: u32,
    pub torrent: TorrentInfo,
    pub config: FakerConfig,
    pub cumulative_uploaded: u64,
    pub cumulative_downloaded: u64,
    pub state: FakerState,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedState {
    pub instances: HashMap<u32, PersistedInstance>,
    pub next_instance_id: u32,
    pub version: u32,
}

impl PersistedState {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            next_instance_id: 1,
            version: 1,
        }
    }
}

fn state_file_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".config")
            .join("rustatio")
            .join("desktop-state.json")
    } else {
        PathBuf::from("desktop-state.json")
    }
}

pub fn load_state() -> PersistedState {
    let path = state_file_path();

    if !path.exists() {
        log::info!("No saved state found at {}, starting fresh", path.display());
        return PersistedState::new();
    }

    match std::fs::read_to_string(&path) {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(state) => {
                let state: PersistedState = state;
                log::info!(
                    "Loaded saved state from {} ({} instances)",
                    path.display(),
                    state.instances.len()
                );
                state
            }
            Err(e) => {
                log::error!("Failed to parse state file: {}", e);
                let backup = path.with_extension("json.corrupted");
                let _ = std::fs::rename(&path, &backup);
                log::warn!("Backed up corrupted state to {}", backup.display());
                PersistedState::new()
            }
        },
        Err(e) => {
            log::error!("Failed to read state file: {}", e);
            PersistedState::new()
        }
    }
}

pub fn save_state(state: &PersistedState) -> Result<(), String> {
    let path = state_file_path();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create data directory: {}", e))?;
    }

    let json = serde_json::to_string_pretty(state).map_err(|e| format!("Failed to serialize state: {}", e))?;

    // Use a unique temp file to avoid races between concurrent save_state calls
    let unique_id = std::process::id();
    let temp_name = format!("desktop-state.json.{}.tmp", unique_id);
    let temp_path = path.with_file_name(temp_name);

    std::fs::write(&temp_path, json.as_bytes()).map_err(|e| format!("Failed to write temp file: {}", e))?;

    // sync via opening the file and syncing
    if let Ok(file) = std::fs::File::open(&temp_path) {
        let _ = file.sync_all();
    }

    std::fs::rename(&temp_path, &path).map_err(|e| {
        // Clean up temp file on rename failure
        let _ = std::fs::remove_file(&temp_path);
        format!("Failed to rename state file: {}", e)
    })?;

    log::debug!("State saved to {}", path.display());
    Ok(())
}

pub fn now_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
