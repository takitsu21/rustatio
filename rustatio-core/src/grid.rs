use crate::faker::{PostStopAction, PresetSettings};
use crate::torrent::ClientType;
use serde::{Deserialize, Serialize};
use url::Url;

fn normalize_tracker_host(host: &str) -> Option<String> {
    let value = host.trim().trim_end_matches('.');
    if value.is_empty() {
        None
    } else {
        Some(value.to_ascii_lowercase())
    }
}

pub fn primary_tracker_host(announce: &str) -> Option<String> {
    let value = announce.trim();
    if value.is_empty() {
        return None;
    }

    if let Ok(parsed) = Url::parse(value) {
        if let Some(host) = parsed.host_str() {
            return normalize_tracker_host(host);
        }
    }

    if !value.contains("://") {
        let fallback = format!("https://{value}");
        if let Ok(parsed) = Url::parse(&fallback) {
            if let Some(host) = parsed.host_str() {
                return normalize_tracker_host(host);
            }
        }
    }

    let head = value.split('/').next();
    let host = head.and_then(|segment| segment.split(':').next());
    host.and_then(normalize_tracker_host)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridMode {
    #[default]
    Seed,
    Leech,
    Custom(f64),
}

impl GridMode {
    pub const fn completion_percent(&self) -> f64 {
        match self {
            Self::Seed => 100.0,
            Self::Leech => 0.0,
            Self::Custom(pct) => pct.clamp(0.0, 100.0),
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
            config.selected_client = Some(*client);
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_tracker_host: Option<String>,
    pub state: String,
    #[serde(default)]
    pub is_tracker_invalid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracker_error: Option<String>,
    #[serde(default)]
    pub tracker_retry_attempt: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracker_retry_at_ms: Option<u64>,
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
    #[serde(default)]
    pub is_idling: bool,
    #[serde(default)]
    pub idling_reason: Option<String>,
    #[serde(default)]
    pub stop_condition_met: bool,
    #[serde(default)]
    pub post_stop_action: PostStopAction,
    #[serde(default)]
    pub effective_stop_at_ratio: Option<f64>,
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
        assert!(settings.tags.is_empty(), "default grid import settings should have no tags");
        assert!(!settings.auto_start);
        assert!(settings.stagger_start_secs.is_none());
    }

    #[test]
    fn test_resolve_for_instance_applies_mode() {
        let settings = GridImportSettings { mode: GridMode::Leech, ..Default::default() };
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

    #[test]
    fn test_primary_tracker_host_extracts_normalized_host() {
        assert_eq!(
            primary_tracker_host("https://Tracker.EXAMPLE.com:443/announce?passkey=abc"),
            Some("tracker.example.com".to_string())
        );
        assert_eq!(
            primary_tracker_host("udp://open.stealth.si:80/announce"),
            Some("open.stealth.si".to_string())
        );
        assert_eq!(
            primary_tracker_host("tracker.torrent.eu.org/announce"),
            Some("tracker.torrent.eu.org".to_string())
        );
        assert_eq!(primary_tracker_host(""), None);
    }

    #[test]
    fn test_instance_summary_serializes_runtime_fields() {
        let summary = InstanceSummary {
            id: "id".to_string(),
            name: "name".to_string(),
            info_hash: "info".to_string(),
            primary_tracker_host: None,
            state: "stopped".to_string(),
            is_tracker_invalid: false,
            tracker_error: None,
            tracker_retry_attempt: 0,
            tracker_retry_at_ms: None,
            tags: Vec::new(),
            total_size: 0,
            uploaded: 0,
            downloaded: 0,
            ratio: 0.0,
            current_upload_rate: 0.0,
            current_download_rate: 0.0,
            seeders: 0,
            leechers: 0,
            left: 0,
            torrent_completion: 100.0,
            source: "manual".to_string(),
            created_at: 0,
            is_idling: true,
            idling_reason: Some("no_leechers".to_string()),
            stop_condition_met: true,
            post_stop_action: PostStopAction::DeleteInstance,
            effective_stop_at_ratio: Some(2.5),
        };

        let value = serde_json::to_value(&summary).unwrap_or(serde_json::Value::Null);
        assert_eq!(value["isIdling"], true);
        assert_eq!(value["idlingReason"], "no_leechers");
        assert_eq!(value["stopConditionMet"], true);
        assert_eq!(value["postStopAction"], "delete_instance");
        assert_eq!(value["effectiveStopAtRatio"], 2.5);
    }
}
