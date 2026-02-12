use crate::torrent::ClientType;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    TomlSerializeError(#[from] toml::ser::Error),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub client: ClientSettings,

    #[serde(default)]
    pub faker: FakerSettings,

    #[serde(default)]
    pub ui: UiSettings,

    #[serde(default)]
    pub instances: Vec<InstanceConfig>,

    #[serde(default)]
    pub active_instance_id: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub torrent_path: Option<String>,
    #[serde(default)]
    pub torrent_name: Option<String>,
    pub selected_client: ClientType,
    pub selected_client_version: Option<String>,
    pub upload_rate: f64,
    pub download_rate: f64,
    pub port: u16,
    pub completion_percent: f64,
    pub initial_uploaded: u64,
    pub initial_downloaded: u64,
    #[serde(default)]
    pub cumulative_uploaded: u64,
    #[serde(default)]
    pub cumulative_downloaded: u64,
    pub randomize_rates: bool,
    pub random_range_percent: f64,
    pub update_interval_seconds: u64,
    pub stop_at_ratio_enabled: bool,
    pub stop_at_ratio: f64,
    pub stop_at_uploaded_enabled: bool,
    pub stop_at_uploaded_gb: f64,
    pub stop_at_downloaded_enabled: bool,
    pub stop_at_downloaded_gb: f64,
    pub stop_at_seed_time_enabled: bool,
    pub stop_at_seed_time_hours: f64,
    pub idle_when_no_leechers: bool,
    pub idle_when_no_seeders: bool,
    pub progressive_rates_enabled: bool,
    pub target_upload_rate: f64,
    pub target_download_rate: f64,
    pub progressive_duration_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSettings {
    /// Default client type to emulate
    #[serde(default = "default_client_type")]
    pub default_type: ClientType,

    /// Default client version (None uses the client's default)
    pub default_version: Option<String>,

    /// Default port
    #[serde(default = "default_port")]
    pub default_port: u16,

    /// Default number of peers to request
    #[serde(default = "default_num_want")]
    pub default_num_want: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FakerSettings {
    /// Default upload rate in KB/s
    #[serde(default = "default_upload_rate")]
    pub default_upload_rate: f64,

    /// Default download rate in KB/s
    #[serde(default = "default_download_rate")]
    pub default_download_rate: f64,

    /// Default announce interval in seconds (if tracker doesn't specify)
    #[serde(default = "default_announce_interval")]
    pub default_announce_interval: u64,

    /// Auto-update stats interval in seconds
    #[serde(default = "default_update_interval")]
    pub update_interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    /// Window width
    #[serde(default = "default_window_width")]
    pub window_width: u32,

    /// Window height
    #[serde(default = "default_window_height")]
    pub window_height: u32,

    /// Enable dark mode
    #[serde(default = "default_dark_mode")]
    pub dark_mode: bool,

    /// Show application logs
    #[serde(default = "default_show_logs")]
    pub show_logs: bool,
}

// Default values
fn default_client_type() -> ClientType {
    ClientType::QBittorrent
}

fn default_port() -> u16 {
    6881
}

fn default_num_want() -> u32 {
    50
}

fn default_upload_rate() -> f64 {
    50.0
}

fn default_download_rate() -> f64 {
    100.0
}

fn default_announce_interval() -> u64 {
    1800 // 30 minutes
}

fn default_update_interval() -> u64 {
    5 // 5 seconds
}

fn default_window_width() -> u32 {
    1200
}

fn default_window_height() -> u32 {
    800
}

fn default_dark_mode() -> bool {
    true
}

fn default_show_logs() -> bool {
    false
}

impl Default for ClientSettings {
    fn default() -> Self {
        ClientSettings {
            default_type: default_client_type(),
            default_version: None,
            default_port: default_port(),
            default_num_want: default_num_want(),
        }
    }
}

impl Default for FakerSettings {
    fn default() -> Self {
        FakerSettings {
            default_upload_rate: default_upload_rate(),
            default_download_rate: default_download_rate(),
            default_announce_interval: default_announce_interval(),
            update_interval: default_update_interval(),
        }
    }
}

impl Default for UiSettings {
    fn default() -> Self {
        UiSettings {
            window_width: default_window_width(),
            window_height: default_window_height(),
            dark_mode: default_dark_mode(),
            show_logs: default_show_logs(),
        }
    }
}

impl AppConfig {
    /// Load configuration from a TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let toml_string = toml::to_string_pretty(self)?;
        fs::write(path, toml_string)?;
        Ok(())
    }

    /// Get the default config file path
    pub fn default_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("rustatio").join("config.toml")
        } else {
            PathBuf::from("rustatio.toml")
        }
    }

    /// Load from default path or create default config if not exists
    pub fn load_or_default() -> Self {
        let path = Self::default_path();

        if path.exists() {
            Self::load(&path).unwrap_or_else(|e| {
                log::warn!("Failed to load config from {:?}: {}. Using defaults.", path, e);
                Self::default()
            })
        } else {
            let config = Self::default();

            // Try to create config directory and save default config
            if let Some(parent) = path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    log::warn!("Failed to create config directory: {}", e);
                }
            }

            if let Err(e) = config.save(&path) {
                log::warn!("Failed to save default config: {}", e);
            } else {
                log::info!("Created default config at {:?}", path);
            }

            config
        }
    }

    /// Create an example config file content
    pub fn example_toml() -> String {
        let config = Self::default();
        toml::to_string_pretty(&config).unwrap_or_default()
    }
}

// Add dirs crate to Cargo.toml for getting config directory
// For now, we'll use a simple implementation

mod dirs {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        if let Ok(home) = std::env::var("HOME") {
            Some(PathBuf::from(home).join(".config"))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.faker.default_upload_rate, 50.0);
        assert_eq!(config.faker.default_download_rate, 100.0);
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let toml = toml::to_string(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml).unwrap();

        assert_eq!(config.faker.default_upload_rate, parsed.faker.default_upload_rate);
    }
}
