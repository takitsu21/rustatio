use crate::faker::PostStopAction;
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
    #[serde(default)]
    pub vpn_port_sync: bool,
    pub completion_percent: f64,
    pub initial_uploaded: u64,
    pub initial_downloaded: u64,
    #[serde(default)]
    pub cumulative_uploaded: u64,
    #[serde(default)]
    pub cumulative_downloaded: u64,
    pub randomize_rates: bool,
    pub random_range_percent: f64,
    #[serde(default)]
    pub randomize_ratio: bool,
    #[serde(default = "default_random_ratio_range_percent")]
    pub random_ratio_range_percent: f64,
    pub update_interval_seconds: u64,
    pub stop_at_ratio_enabled: bool,
    pub stop_at_ratio: f64,
    #[serde(default)]
    pub effective_stop_at_ratio: Option<f64>,
    pub stop_at_uploaded_enabled: bool,
    pub stop_at_uploaded_gb: f64,
    pub stop_at_downloaded_enabled: bool,
    pub stop_at_downloaded_gb: f64,
    pub stop_at_seed_time_enabled: bool,
    pub stop_at_seed_time_hours: f64,
    pub idle_when_no_leechers: bool,
    pub idle_when_no_seeders: bool,
    #[serde(default)]
    pub post_stop_action: PostStopAction,
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
const fn default_client_type() -> ClientType {
    ClientType::QBittorrent
}

const fn default_port() -> u16 {
    6881
}

const fn default_num_want() -> u32 {
    50
}

const fn default_upload_rate() -> f64 {
    50.0
}

const fn default_download_rate() -> f64 {
    100.0
}

const fn default_random_ratio_range_percent() -> f64 {
    10.0
}

const fn default_announce_interval() -> u64 {
    1800 // 30 minutes
}

const fn default_update_interval() -> u64 {
    5 // 5 seconds
}

const fn default_window_width() -> u32 {
    1200
}

const fn default_window_height() -> u32 {
    800
}

const fn default_dark_mode() -> bool {
    true
}

const fn default_show_logs() -> bool {
    false
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            default_type: default_client_type(),
            default_version: None,
            default_port: default_port(),
            default_num_want: default_num_want(),
        }
    }
}

impl Default for FakerSettings {
    fn default() -> Self {
        Self {
            default_upload_rate: default_upload_rate(),
            default_download_rate: default_download_rate(),
            default_announce_interval: default_announce_interval(),
            update_interval: default_update_interval(),
        }
    }
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
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
        let config: Self = toml::from_str(&content)?;
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
        dirs::config_dir().map_or_else(
            || PathBuf::from("rustatio.toml"),
            |config_dir| config_dir.join("rustatio").join("config.toml"),
        )
    }

    /// Load from default path or create default config if not exists
    pub fn load_or_default() -> Self {
        let path = Self::default_path();

        if path.exists() {
            Self::load(&path).unwrap_or_else(|e| {
                log::warn!("Failed to load config from {}: {e}. Using defaults.", path.display());
                Self::default()
            })
        } else {
            let config = Self::default();

            // Try to create config directory and save default config
            if let Some(parent) = path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    log::warn!("Failed to create config directory: {e}");
                }
            }

            if let Err(e) = config.save(&path) {
                log::warn!("Failed to save default config: {e}");
            } else {
                log::info!("Created default config at {}", path.display());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(name);
        path
    }

    fn set_env(key: &str, value: Option<&str>) {
        match value {
            Some(val) => std::env::set_var(key, val),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.faker.default_upload_rate, 50.0);
        assert_eq!(config.faker.default_download_rate, 100.0);
        assert_eq!(config.client.default_port, 6881);
        assert_eq!(config.client.default_num_want, 50);
        assert_eq!(config.ui.window_width, 1200);
        assert_eq!(config.ui.window_height, 800);
        assert!(config.ui.dark_mode);
        assert!(!config.ui.show_logs);
        assert!(config.instances.is_empty());
        assert!(config.active_instance_id.is_none());
    }

    #[test]
    fn test_config_serialization() -> Result<()> {
        let config = AppConfig::default();
        let toml = toml::to_string(&config).map_err(ConfigError::TomlSerializeError)?;
        let parsed: AppConfig = toml::from_str(&toml).map_err(ConfigError::TomlError)?;

        assert_eq!(config.faker.default_upload_rate, parsed.faker.default_upload_rate);
        Ok(())
    }

    #[test]
    fn test_save_and_load_roundtrip() -> Result<()> {
        let config = AppConfig::default();
        let path = temp_path("rustatio_config_test.toml");
        config.save(&path)?;

        let loaded = AppConfig::load(&path)?;
        assert_eq!(loaded.client.default_port, 6881);
        assert_eq!(loaded.faker.default_upload_rate, 50.0);

        std::fs::remove_file(&path)?;
        Ok(())
    }

    #[test]
    fn test_load_invalid_toml() -> Result<()> {
        let path = temp_path("rustatio_bad_config.toml");
        std::fs::write(&path, "[not toml")?;
        let result = AppConfig::load(&path);

        assert!(matches!(result, Err(ConfigError::TomlError(_))));
        std::fs::remove_file(&path)?;
        Ok(())
    }

    #[test]
    fn test_default_path_with_home() {
        let _guard = ENV_LOCK.lock();
        let current_home = std::env::var("HOME").ok();
        let current_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let temp = std::env::temp_dir();
        let temp_str = temp.to_string_lossy().to_string();
        set_env("HOME", Some(&temp_str));
        set_env("XDG_CONFIG_HOME", Some(&temp_str));

        let path = AppConfig::default_path();
        assert_eq!(path, temp.join("rustatio").join("config.toml"));

        set_env("HOME", current_home.as_deref());
        set_env("XDG_CONFIG_HOME", current_xdg.as_deref());
    }

    #[test]
    fn test_default_path_without_home() {
        let _guard = ENV_LOCK.lock();
        let current_home = std::env::var("HOME").ok();
        let current_xdg = std::env::var("XDG_CONFIG_HOME").ok();

        let temp = std::env::temp_dir();
        let temp_str = temp.to_string_lossy().to_string();
        set_env("HOME", Some(&temp_str));
        set_env("XDG_CONFIG_HOME", None);

        let path = AppConfig::default_path();
        assert_eq!(path, temp.join(".config").join("rustatio").join("config.toml"));

        set_env("HOME", current_home.as_deref());
        set_env("XDG_CONFIG_HOME", current_xdg.as_deref());
    }

    #[test]
    fn test_load_or_default_creates_file() -> Result<()> {
        let _guard = ENV_LOCK.lock();
        let current_home = std::env::var("HOME").ok();
        let current_xdg = std::env::var("XDG_CONFIG_HOME").ok();

        let temp = temp_path("rustatio_config_home");
        std::fs::create_dir_all(&temp)?;
        let temp_str = temp.to_string_lossy().to_string();
        set_env("HOME", Some(&temp_str));
        set_env("XDG_CONFIG_HOME", Some(&temp_str));

        let config = AppConfig::load_or_default();
        assert_eq!(config.client.default_port, 6881);

        let path = AppConfig::default_path();
        assert!(path.exists());

        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.is_file() {
                let _ = std::fs::remove_file(&path);
            }
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }

        set_env("HOME", current_home.as_deref());
        set_env("XDG_CONFIG_HOME", current_xdg.as_deref());
        Ok(())
    }

    #[test]
    fn test_load_or_default_invalid_file() -> Result<()> {
        let _guard = ENV_LOCK.lock();
        let current_home = std::env::var("HOME").ok();
        let current_xdg = std::env::var("XDG_CONFIG_HOME").ok();

        let temp = temp_path("rustatio_config_bad_home");
        std::fs::create_dir_all(&temp)?;
        let temp_str = temp.to_string_lossy().to_string();
        set_env("HOME", Some(&temp_str));
        set_env("XDG_CONFIG_HOME", Some(&temp_str));

        let path = AppConfig::default_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, "not toml")?;

        let config = AppConfig::load_or_default();
        assert_eq!(config.faker.default_download_rate, 100.0);

        let _ = std::fs::remove_file(&path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }

        set_env("HOME", current_home.as_deref());
        set_env("XDG_CONFIG_HOME", current_xdg.as_deref());
        Ok(())
    }

    #[test]
    fn test_example_toml_parses() -> Result<()> {
        let text = AppConfig::example_toml();
        let parsed: AppConfig = toml::from_str(&text).map_err(ConfigError::TomlError)?;
        assert_eq!(parsed.client.default_port, 6881);
        Ok(())
    }

    #[test]
    fn test_instance_defaults_from_toml() -> Result<()> {
        let input = r#"
            [[instances]]
            torrent_path = "path"
            selected_client = "qbittorrent"
            upload_rate = 1.0
            download_rate = 2.0
            port = 6881
            vpn_port_sync = false
            completion_percent = 0.0
            initial_uploaded = 0
            initial_downloaded = 0
            randomize_rates = false
            random_range_percent = 0.0
            update_interval_seconds = 10
            stop_at_ratio_enabled = false
            stop_at_ratio = 0.0
            stop_at_uploaded_enabled = false
            stop_at_uploaded_gb = 0.0
            stop_at_downloaded_enabled = false
            stop_at_downloaded_gb = 0.0
            stop_at_seed_time_enabled = false
            stop_at_seed_time_hours = 0.0
            idle_when_no_leechers = false
            idle_when_no_seeders = false
            progressive_rates_enabled = false
            target_upload_rate = 0.0
            target_download_rate = 0.0
            progressive_duration_hours = 0.0
        "#;
        let config: AppConfig = toml::from_str(input).map_err(ConfigError::TomlError)?;
        let inst = &config.instances[0];
        assert!(inst.torrent_name.is_none());
        assert_eq!(inst.cumulative_uploaded, 0);
        assert_eq!(inst.cumulative_downloaded, 0);
        assert!(!inst.vpn_port_sync);
        Ok(())
    }
}
