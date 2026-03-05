use crate::persistence::WatchSettings;
use crate::state::{now_secs, AppState, FakerInstance};
use rustatio_core::{FakerConfig, FakerState, RatioFaker, RatioFakerHandle};
use rustatio_watch::{
    EngineConfig, InstanceSource, InstanceState, NewInstance, WatchEngine, WatchService,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type DesktopWatchService = WatchService<DesktopWatchEngine>;

pub struct DesktopWatchEngine {
    state: AppState,
    default_config: Arc<RwLock<Option<FakerConfig>>>,
}

impl DesktopWatchEngine {
    pub const fn new(state: AppState, default_config: Arc<RwLock<Option<FakerConfig>>>) -> Self {
        Self { state, default_config }
    }
}

#[async_trait::async_trait]
impl WatchEngine for DesktopWatchEngine {
    async fn list_instances(&self) -> Vec<InstanceState> {
        let fakers = self.state.fakers.read().await;
        fakers
            .iter()
            .map(|(id, instance)| {
                let stats = instance.faker.stats_snapshot();
                let state = match stats.state {
                    FakerState::Paused => "paused",
                    _ if stats.is_idling => "idle",
                    FakerState::Idle => "idle",
                    FakerState::Starting => "starting",
                    FakerState::Running => "running",
                    FakerState::Stopping => "stopping",
                    FakerState::Stopped => "stopped",
                };

                InstanceState {
                    id: id.to_string(),
                    info_hash: instance.summary.info_hash,
                    source: instance.source,
                    state: state.to_string(),
                    name: instance.summary.name.clone(),
                }
            })
            .collect()
    }

    async fn create_instance(&self, instance: NewInstance) -> Result<(), String> {
        let faker = RatioFaker::new(
            Arc::new(instance.info.clone()),
            instance.config.clone(),
            Some(self.state.http_client.clone()),
        )
        .map_err(|e| format!("Failed to create faker: {e}"))?;

        let now = now_secs();
        let summary = Arc::new(instance.info.summary());
        let torrent = Arc::new(instance.info);

        let id = instance.id.parse::<u32>().map_err(|_| "Invalid instance id")?;

        let mut fakers = self.state.fakers.write().await;
        fakers.insert(
            id,
            FakerInstance {
                faker: Arc::new(RatioFakerHandle::new(faker)),
                torrent,
                summary,
                config: instance.config,
                cumulative_uploaded: 0,
                cumulative_downloaded: 0,
                tags: vec![],
                created_at: now,
                source: InstanceSource::WatchFolder,
            },
        );

        Ok(())
    }

    async fn start_instance(&self, id: &str) -> Result<(), String> {
        let instance_id = id.parse::<u32>().map_err(|_| "Invalid instance id")?;
        let faker = {
            let fakers = self.state.fakers.read().await;
            let instance = fakers.get(&instance_id).ok_or("Instance not found")?;
            Arc::clone(&instance.faker)
        };

        faker.start().await.map_err(|e| e.to_string())
    }

    async fn delete_instance_by_info_hash(&self, info_hash: &[u8; 20]) -> Result<(), String> {
        let target = {
            let fakers = self.state.fakers.read().await;
            fakers
                .iter()
                .find(|(_, instance)| instance.summary.info_hash == *info_hash)
                .map(|(id, _)| *id)
        };

        if let Some(id) = target {
            let removed = {
                let mut fakers = self.state.fakers.write().await;
                fakers.remove(&id)
            };

            if let Some(instance) = removed {
                let _ = instance.faker.stop().await;
            }
        }

        Ok(())
    }

    async fn find_instance_by_info_hash(&self, info_hash: &[u8; 20]) -> Option<String> {
        let fakers = self.state.fakers.read().await;
        fakers
            .iter()
            .find(|(_, instance)| instance.summary.info_hash == *info_hash)
            .map(|(id, _)| id.to_string())
    }

    async fn update_instance_source_by_info_hash(
        &self,
        info_hash: &[u8; 20],
        source: InstanceSource,
    ) -> Result<(), String> {
        let mut fakers = self.state.fakers.write().await;
        let entry = fakers
            .iter_mut()
            .find(|(_, instance)| instance.summary.info_hash == *info_hash)
            .map(|(_, instance)| instance);

        let Some(instance) = entry else {
            return Ok(());
        };

        instance.source = source;
        Ok(())
    }

    async fn default_config(&self) -> Option<FakerConfig> {
        let config = self.default_config.read().await;
        config.clone()
    }

    fn next_instance_id(&self) -> String {
        let next_id = Arc::clone(&self.state.next_instance_id);
        let result = tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                let mut id = next_id.write().await;
                let current = *id;
                *id = id.saturating_add(1);
                current.to_string()
            })
        });
        result
    }
}

pub fn default_watch_dir(settings: Option<&WatchSettings>) -> PathBuf {
    if let Some(path) = settings.and_then(|s| s.watch_dir.as_deref()) {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    std::env::var("WATCH_DIR").map_or_else(|_| PathBuf::from("torrents"), PathBuf::from)
}

pub fn build_watch_service(
    state: AppState,
    defaults: Arc<RwLock<Option<FakerConfig>>>,
    settings: Option<WatchSettings>,
) -> DesktopWatchService {
    let watch_dir = default_watch_dir(settings.as_ref());
    let watch_settings = settings.unwrap_or_default();
    let config = EngineConfig {
        watch_dir,
        auto_start: watch_settings.auto_start,
        enabled: true,
        max_depth: watch_settings.max_depth,
    };

    WatchService::new(config, Arc::new(DesktopWatchEngine::new(state, defaults)))
}
