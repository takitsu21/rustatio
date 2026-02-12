use crate::faker::PresetSettings;
use crate::torrent::ClientType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridMode {
    #[default]
    Seed,
    Leech,
    Custom(f64),
}

impl GridMode {
    pub fn completion_percent(&self) -> f64 {
        match self {
            GridMode::Seed => 100.0,
            GridMode::Leech => 0.0,
            GridMode::Custom(pct) => pct.clamp(0.0, 100.0),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridImportSettings {
    #[serde(default)]
    pub base_config: PresetSettings,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub mode: GridMode,
    #[serde(default)]
    pub auto_start: bool,
    pub stagger_start_secs: Option<u64>,
    pub client_type: Option<ClientType>,
    pub client_version: Option<String>,
}

impl GridImportSettings {
    pub fn resolve_for_instance(&self) -> PresetSettings {
        let mut config = self.base_config.clone();

        config.completion_percent = Some(self.mode.completion_percent());

        if let Some(ref client) = self.client_type {
            config.selected_client = Some(client.clone());
        }

        if let Some(ref version) = self.client_version {
            config.selected_client_version = Some(version.clone());
        }

        config
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSummary {
    pub id: String,
    pub name: String,
    pub info_hash: String,
    pub state: String,
    pub tags: Vec<String>,
    pub total_size: u64,
    pub uploaded: u64,
    pub downloaded: u64,
    pub ratio: f64,
    pub current_upload_rate: f64,
    pub current_download_rate: f64,
    pub seeders: i64,
    pub leechers: i64,
    pub left: u64,
    pub torrent_completion: f64,
    pub source: String,
    pub created_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_mode_completion_percent() {
        assert_eq!(GridMode::Seed.completion_percent(), 100.0);
        assert_eq!(GridMode::Leech.completion_percent(), 0.0);
        assert_eq!(GridMode::Custom(75.0).completion_percent(), 75.0);
        assert_eq!(GridMode::Custom(150.0).completion_percent(), 100.0);
        assert_eq!(GridMode::Custom(-10.0).completion_percent(), 0.0);
    }

    #[test]
    fn test_grid_import_settings_default() {
        let settings = GridImportSettings::default();
        assert!(settings.tags.is_empty());
        assert!(!settings.auto_start);
        assert!(settings.stagger_start_secs.is_none());
    }

    #[test]
    fn test_resolve_for_instance_applies_mode() {
        let settings = GridImportSettings {
            mode: GridMode::Leech,
            ..Default::default()
        };
        let resolved = settings.resolve_for_instance();
        assert_eq!(resolved.completion_percent, Some(0.0));
    }

    #[test]
    fn test_resolve_for_instance_uses_base_config_rates() {
        let settings = GridImportSettings {
            base_config: PresetSettings {
                upload_rate: Some(100.0),
                download_rate: Some(50.0),
                ..Default::default()
            },
            ..Default::default()
        };
        let resolved = settings.resolve_for_instance();
        assert_eq!(resolved.upload_rate, Some(100.0));
        assert_eq!(resolved.download_rate, Some(50.0));
    }

    #[test]
    fn test_resolve_for_instance_applies_client() {
        let settings = GridImportSettings {
            client_type: Some(ClientType::Transmission),
            client_version: Some("4.0.0".to_string()),
            ..Default::default()
        };
        let resolved = settings.resolve_for_instance();
        assert_eq!(resolved.selected_client, Some(ClientType::Transmission));
        assert_eq!(resolved.selected_client_version, Some("4.0.0".to_string()));
    }
}
