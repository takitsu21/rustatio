use rustatio_core::{FakerConfig, FakerStats, RatioFaker, TorrentInfo};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Instance data with cumulative stats tracking
pub struct FakerInstance {
    pub faker: RatioFaker,
    pub torrent_info_hash: [u8; 20],
    pub cumulative_uploaded: u64,
    pub cumulative_downloaded: u64,
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
}

impl AppState {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            torrents: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
        }
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
        let torrent_info_hash = torrent.info_hash;

        // Check if instance exists and has same torrent - preserve cumulative stats
        let (cumulative_uploaded, cumulative_downloaded) = {
            let instances = self.instances.read().await;
            if let Some(existing) = instances.get(id) {
                if existing.torrent_info_hash == torrent_info_hash {
                    (existing.cumulative_uploaded, existing.cumulative_downloaded)
                } else {
                    (0, 0)
                }
            } else {
                (0, 0)
            }
        };

        // Apply cumulative stats to config
        let mut config = config;
        config.initial_uploaded = cumulative_uploaded;
        config.initial_downloaded = cumulative_downloaded;

        let faker = RatioFaker::new(torrent, config).map_err(|e| e.to_string())?;

        let instance = FakerInstance {
            faker,
            torrent_info_hash,
            cumulative_uploaded,
            cumulative_downloaded,
        };

        self.instances.write().await.insert(id.to_string(), instance);
        Ok(())
    }

    /// Start a faker instance
    pub async fn start_instance(&self, id: &str) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;
        instance.faker.start().await.map_err(|e| e.to_string())
    }

    /// Stop a faker instance
    pub async fn stop_instance(&self, id: &str) -> Result<FakerStats, String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;

        // Get final stats before stopping
        let stats = instance.faker.get_stats().await;

        // Update cumulative stats
        instance.cumulative_uploaded = stats.uploaded;
        instance.cumulative_downloaded = stats.downloaded;

        instance.faker.stop().await.map_err(|e| e.to_string())?;
        Ok(stats)
    }

    /// Pause a faker instance
    pub async fn pause_instance(&self, id: &str) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;
        instance.faker.pause().await.map_err(|e| e.to_string())
    }

    /// Resume a faker instance
    pub async fn resume_instance(&self, id: &str) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;
        instance.faker.resume().await.map_err(|e| e.to_string())
    }

    /// Update faker (send tracker announce)
    pub async fn update_instance(&self, id: &str) -> Result<FakerStats, String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;
        instance.faker.update().await.map_err(|e| e.to_string())?;
        Ok(instance.faker.get_stats().await)
    }

    /// Update stats only (no tracker announce)
    pub async fn update_stats_only(&self, id: &str) -> Result<FakerStats, String> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(id).ok_or("Instance not found")?;
        instance.faker.update_stats_only().await.map_err(|e| e.to_string())?;
        Ok(instance.faker.get_stats().await)
    }

    /// Get stats for an instance
    pub async fn get_stats(&self, id: &str) -> Result<FakerStats, String> {
        let instances = self.instances.read().await;
        let instance = instances.get(id).ok_or("Instance not found")?;
        Ok(instance.faker.get_stats().await)
    }

    /// Delete an instance
    pub async fn delete_instance(&self, id: &str) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        if instances.remove(id).is_some() {
            Ok(())
        } else {
            Err("Instance not found".to_string())
        }
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
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
