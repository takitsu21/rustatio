use rustatio_core::{FakerConfig, FakerState, TorrentSummary};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstanceSource {
    #[default]
    Manual,
    WatchFolder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedInstance {
    pub id: String,
    pub torrent: TorrentSummary,
    pub config: FakerConfig,
    pub cumulative_uploaded: u64,
    pub cumulative_downloaded: u64,
    pub state: FakerState,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub source: InstanceSource,
    #[serde(default)]
    pub tags: Vec<String>,
}

const fn default_watch_max_depth() -> u32 {
    1
}

fn default_watch_auto_start() -> bool {
    std::env::var("WATCH_AUTO_START")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false)
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct WatchSettings {
    #[serde(default = "default_watch_max_depth")]
    pub max_depth: u32,
    #[serde(default = "default_watch_auto_start")]
    pub auto_start: bool,
}

impl Default for WatchSettings {
    fn default() -> Self {
        Self { max_depth: default_watch_max_depth(), auto_start: default_watch_auto_start() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedState {
    pub instances: HashMap<String, PersistedInstance>,
    #[serde(default)]
    pub default_config: Option<FakerConfig>,
    #[serde(default)]
    pub watch_settings: Option<WatchSettings>,
    pub version: u32,
}

impl PersistedState {
    pub fn new() -> Self {
        Self { instances: HashMap::new(), default_config: None, watch_settings: None, version: 1 }
    }
}

pub struct Persistence {
    state_file: String,
}

impl Persistence {
    pub fn new(data_dir: &str) -> Self {
        Self { state_file: format!("{data_dir}/state.json") }
    }

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

    pub async fn save(&self, state: &PersistedState) -> Result<(), String> {
        if let Some(parent) = Path::new(&self.state_file).parent() {
            if let Err(e) = fs::create_dir_all(parent).await {
                return Err(format!("Failed to create data directory: {e}"));
            }
        }

        let temp_file = format!("{}.tmp", self.state_file);

        let mut file = fs::File::create(&temp_file)
            .await
            .map_err(|e| format!("Failed to create temp file: {e}"))?;

        let json =
            serde_json::to_vec(state).map_err(|e| format!("Failed to serialize state: {e}"))?;

        file.write_all(&json).await.map_err(|e| format!("Failed to write state: {e}"))?;

        file.sync_all().await.map_err(|e| format!("Failed to sync state file: {e}"))?;

        fs::rename(&temp_file, &self.state_file)
            .await
            .map_err(|e| format!("Failed to rename state file: {e}"))?;

        tracing::debug!("State saved to {}", self.state_file);
        Ok(())
    }
}

pub fn now_timestamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}

#[cfg(test)]
mod tests {
    use super::WatchSettings;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn watch_settings_default_uses_env_auto_start() {
        let guard = env_lock().lock();
        assert!(guard.is_ok(), "failed to acquire env mutex");

        std::env::set_var("WATCH_AUTO_START", "1");
        let settings = WatchSettings::default();
        assert!(settings.auto_start);

        std::env::set_var("WATCH_AUTO_START", "false");
        let settings = WatchSettings::default();
        assert!(!settings.auto_start);

        std::env::remove_var("WATCH_AUTO_START");
    }
}
