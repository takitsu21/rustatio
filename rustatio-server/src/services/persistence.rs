use rustatio_core::{FakerConfig, FakerState, TorrentSummary};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
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
    #[serde(default)]
    pub runtime: Option<PersistedRuntime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedRuntime {
    pub uploaded: u64,
    pub downloaded: u64,
    pub ratio: f64,
    pub left: u64,
    pub torrent_completion: f64,
    pub seeders: i64,
    pub leechers: i64,
    pub session_uploaded: u64,
    pub session_downloaded: u64,
    pub session_ratio: f64,
    pub elapsed_secs: u64,
    pub current_upload_rate: f64,
    pub current_download_rate: f64,
    pub average_upload_rate: f64,
    pub average_download_rate: f64,
    pub upload_progress: f64,
    pub download_progress: f64,
    pub ratio_progress: f64,
    pub seed_time_progress: f64,
    pub effective_stop_at_ratio: Option<f64>,
    pub eta_ratio_secs: Option<u64>,
    pub eta_uploaded_secs: Option<u64>,
    pub eta_seed_time_secs: Option<u64>,
    pub eta_download_completion_secs: Option<u64>,
    pub stop_condition_met: bool,
    pub is_idling: bool,
    pub idling_reason: Option<String>,
    #[serde(default)]
    pub tracker_error: Option<String>,
    pub announce_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CustomPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    #[serde(default)]
    pub custom: bool,
    #[serde(alias = "createdAt")]
    pub created_at: String,
    #[schema(value_type = Object)]
    pub settings: rustatio_core::PresetSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DefaultPreset {
    pub id: String,
    pub name: String,
    #[schema(value_type = Object)]
    pub settings: rustatio_core::PresetSettings,
}

const fn default_watch_max_depth() -> u32 {
    1
}

fn default_watch_auto_start() -> bool {
    std::env::var("WATCH_AUTO_START").is_ok_and(|v| v.eq_ignore_ascii_case("true") || v == "1")
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
    pub default_preset: Option<DefaultPreset>,
    #[serde(default)]
    pub watch_settings: Option<WatchSettings>,
    #[serde(default)]
    pub custom_presets: Vec<CustomPreset>,
    pub version: u32,
}

impl PersistedState {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            default_config: None,
            default_preset: None,
            watch_settings: None,
            custom_presets: Vec::new(),
            version: 1,
        }
    }
}

pub struct Persistence {
    state_file: String,
    /// Serializes writes so concurrent callers can never share or clobber a
    /// temp file. Before this lock, a bulk delete persisted once per instance
    /// and the interleaved writes could corrupt `state.json`, wiping every
    /// instance on the next load (#166).
    write_lock: tokio::sync::Mutex<()>,
}

impl Persistence {
    pub fn new(data_dir: &str) -> Self {
        Self {
            state_file: format!("{data_dir}/state.json"),
            write_lock: tokio::sync::Mutex::new(()),
        }
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
                        tracing::error!(
                            "Failed to parse state file {}: {}. Starting with an empty state; \
                             instances will look deleted until the file is repaired.",
                            self.state_file,
                            e
                        );
                        let backup = format!("{}.corrupted.{}", self.state_file, now_timestamp());
                        match fs::rename(path, &backup).await {
                            Ok(()) => tracing::error!(
                                "Corrupted state backed up to {}. Move it back after repairing it to restore instances.",
                                backup
                            ),
                            Err(rename_err) => tracing::error!(
                                "Failed to back up corrupted state to {}: {}",
                                backup,
                                rename_err
                            ),
                        }
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
        // Hold the lock for the whole write+rename so two callers can never
        // share the temp file or rename each other's partial output (#166).
        let _guard = self.write_lock.lock().await;

        if let Some(parent) = Path::new(&self.state_file).parent() {
            if let Err(e) = fs::create_dir_all(parent).await {
                return Err(format!("Failed to create data directory: {e}"));
            }
        }

        // A unique temp name is defense in depth on top of the lock: even if a
        // future caller bypasses the lock, writers never touch the same path.
        let temp_file = Self::temp_path(&self.state_file);

        if let Err(e) = Self::write_state(&temp_file, state).await {
            if let Err(cleanup_err) = fs::remove_file(&temp_file).await {
                tracing::debug!("Failed to remove temp state file {}: {}", temp_file, cleanup_err);
            }
            return Err(e);
        }

        if let Err(e) = fs::rename(&temp_file, &self.state_file).await {
            let _ = fs::remove_file(&temp_file).await;
            return Err(format!("Failed to rename state file: {e}"));
        }

        tracing::debug!("State saved to {}", self.state_file);
        Ok(())
    }

    async fn write_state(temp_file: &str, state: &PersistedState) -> Result<(), String> {
        let mut file = fs::File::create(temp_file)
            .await
            .map_err(|e| format!("Failed to create temp file: {e}"))?;

        let json =
            serde_json::to_vec(state).map_err(|e| format!("Failed to serialize state: {e}"))?;

        file.write_all(&json).await.map_err(|e| format!("Failed to write state: {e}"))?;

        file.sync_all().await.map_err(|e| format!("Failed to sync state file: {e}"))?;

        Ok(())
    }

    fn temp_path(state_file: &str) -> String {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("{state_file}.{}.{}.tmp", std::process::id(), seq)
    }
}

pub fn now_timestamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}

#[cfg(test)]
mod tests {
    use super::{InstanceSource, PersistedInstance, PersistedState, Persistence, WatchSettings};
    use rustatio_core::{FakerConfig, FakerState, TorrentInfo};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};

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

    fn sample_state(count: usize) -> PersistedState {
        let mut instances = HashMap::new();
        for i in 0..count {
            let id = format!("inst-{i}");
            let torrent = TorrentInfo {
                info_hash: [u8::try_from(i % 256).unwrap_or(0); 20],
                announce: "https://tracker.test/announce".to_string(),
                announce_list: None,
                name: format!("sample-{i}"),
                total_size: 1024,
                piece_length: 256,
                num_pieces: 4,
                creation_date: None,
                comment: None,
                created_by: None,
                is_single_file: true,
                file_count: 1,
                files: Vec::new(),
            };
            instances.insert(
                id.clone(),
                PersistedInstance {
                    id,
                    torrent: torrent.summary(),
                    config: FakerConfig::default(),
                    cumulative_uploaded: 0,
                    cumulative_downloaded: 0,
                    state: FakerState::Stopped,
                    created_at: 0,
                    updated_at: 0,
                    source: InstanceSource::Manual,
                    tags: Vec::new(),
                    runtime: None,
                },
            );
        }

        PersistedState {
            instances,
            default_config: None,
            default_preset: None,
            watch_settings: None,
            custom_presets: Vec::new(),
            version: 1,
        }
    }

    #[tokio::test]
    async fn concurrent_saves_never_corrupt_state_file() {
        let temp = tempfile::tempdir();
        assert!(temp.is_ok(), "failed to create tempdir");
        let Ok(temp) = temp else {
            return;
        };
        let persistence = Arc::new(Persistence::new(&temp.path().to_string_lossy()));
        let state = sample_state(200);

        let mut handles = Vec::new();
        for _ in 0..50 {
            let persistence = Arc::clone(&persistence);
            let state = state.clone();
            handles.push(tokio::spawn(async move { persistence.save(&state).await }));
        }

        for handle in handles {
            let joined = handle.await;
            assert!(joined.is_ok(), "save task should not panic");
            if let Ok(saved) = joined {
                assert!(saved.is_ok(), "every save should succeed");
            }
        }

        let loaded = persistence.load().await;
        assert_eq!(loaded.instances.len(), 200, "state file must never parse as empty/partial");
    }
}
