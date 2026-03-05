use crate::services::lifecycle::InstanceLifecycle;
use crate::services::persistence::InstanceSource;
use crate::services::state::AppState;
use rustatio_watch::{
    EngineConfig, InstanceSource as WatchSource, InstanceState, NewInstance, WatchEngine,
    WatchService as EngineWatchService,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use utoipa::ToSchema;

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name).map(|v| v.eq_ignore_ascii_case("true") || v == "1").unwrap_or(default)
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WatchStatus {
    pub enabled: bool,
    pub watch_dir: String,
    pub auto_start: bool,
    pub file_count: usize,
    pub loaded_count: usize,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WatchedFile {
    pub filename: String,
    pub path: String,
    pub status: WatchedFileStatus,
    pub info_hash: Option<String>,
    pub name: Option<String>,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum WatchedFileStatus {
    Pending,
    Loaded,
    Invalid,
}

impl From<rustatio_watch::WatchStatus> for WatchStatus {
    fn from(status: rustatio_watch::WatchStatus) -> Self {
        Self {
            enabled: status.enabled,
            watch_dir: status.watch_dir,
            auto_start: status.auto_start,
            file_count: status.file_count,
            loaded_count: status.loaded_count,
        }
    }
}

impl From<rustatio_watch::WatchedFileStatus> for WatchedFileStatus {
    fn from(status: rustatio_watch::WatchedFileStatus) -> Self {
        match status {
            rustatio_watch::WatchedFileStatus::Pending => Self::Pending,
            rustatio_watch::WatchedFileStatus::Loaded => Self::Loaded,
            rustatio_watch::WatchedFileStatus::Invalid => Self::Invalid,
        }
    }
}

impl From<rustatio_watch::WatchedFile> for WatchedFile {
    fn from(file: rustatio_watch::WatchedFile) -> Self {
        Self {
            filename: file.filename,
            path: file.path,
            status: file.status.into(),
            info_hash: file.info_hash,
            name: file.name,
            size: file.size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WatchConfig {
    pub watch_dir: PathBuf,
    pub auto_start: bool,
    pub enabled: bool,
    pub max_depth: u32,
}

#[derive(Debug, Clone)]
pub enum WatchDisabledReason {
    ExplicitlyDisabled,
    DirectoryNotFound,
}

impl WatchConfig {
    pub fn from_env() -> (Self, Option<WatchDisabledReason>) {
        let watch_dir = std::env::var("WATCH_DIR").unwrap_or_else(|_| "/torrents".to_string());
        let watch_path = PathBuf::from(&watch_dir);

        let auto_start = env_bool("WATCH_AUTO_START", false);

        let max_depth = std::env::var("WATCH_MAX_DEPTH")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(1);

        let auto_detect = || {
            if watch_path.exists() && watch_path.is_dir() {
                (true, None)
            } else {
                (false, Some(WatchDisabledReason::DirectoryNotFound))
            }
        };

        let (enabled, disabled_reason) = std::env::var("WATCH_ENABLED").map_or_else(
            |_| auto_detect(),
            |val| {
                let val_lower = val.to_lowercase();
                if val_lower == "false" || val == "0" {
                    (false, Some(WatchDisabledReason::ExplicitlyDisabled))
                } else if val_lower == "true" || val == "1" {
                    (true, None)
                } else if val.is_empty() {
                    auto_detect()
                } else {
                    (true, None)
                }
            },
        );

        (Self { watch_dir: watch_path, auto_start, enabled, max_depth }, disabled_reason)
    }
}

pub struct ServerWatchEngine {
    state: AppState,
}

impl ServerWatchEngine {
    pub const fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait::async_trait]
impl WatchEngine for ServerWatchEngine {
    async fn list_instances(&self) -> Vec<InstanceState> {
        let instances = self.state.list_instances().await;
        instances
            .into_iter()
            .map(|inst| InstanceState {
                id: inst.id,
                info_hash: inst.torrent.info_hash,
                source: match inst.source {
                    InstanceSource::Manual => WatchSource::Manual,
                    InstanceSource::WatchFolder => WatchSource::WatchFolder,
                },
                state: match inst.stats.state {
                    rustatio_core::FakerState::Paused => "paused",
                    _ if inst.stats.is_idling => "idle",
                    rustatio_core::FakerState::Idle => "idle",
                    rustatio_core::FakerState::Starting => "starting",
                    rustatio_core::FakerState::Running => "running",
                    rustatio_core::FakerState::Stopping => "stopping",
                    rustatio_core::FakerState::Stopped => "stopped",
                }
                .to_string(),
                name: inst.torrent.name.clone(),
            })
            .collect()
    }

    async fn create_instance(&self, instance: NewInstance) -> Result<(), String> {
        self.state
            .create_instance_with_event(
                &instance.id,
                instance.info,
                instance.config,
                instance.auto_start,
            )
            .await
    }

    async fn start_instance(&self, id: &str) -> Result<(), String> {
        self.state.start_instance(id).await
    }

    async fn delete_instance_by_info_hash(&self, info_hash: &[u8; 20]) -> Result<(), String> {
        self.state.delete_instance_by_info_hash(info_hash).await
    }

    async fn find_instance_by_info_hash(&self, info_hash: &[u8; 20]) -> Option<String> {
        self.state.find_instance_by_info_hash(info_hash).await
    }

    async fn update_instance_source_by_info_hash(
        &self,
        info_hash: &[u8; 20],
        source: WatchSource,
    ) -> Result<(), String> {
        let mapped = match source {
            WatchSource::Manual => InstanceSource::Manual,
            WatchSource::WatchFolder => InstanceSource::WatchFolder,
        };
        self.state.update_instance_source_by_info_hash(info_hash, mapped).await
    }

    async fn default_config(&self) -> Option<rustatio_core::FakerConfig> {
        self.state.get_default_config().await
    }

    fn next_instance_id(&self) -> String {
        self.state.next_instance_id()
    }
}

pub struct WatchServiceWrapper {
    inner: EngineWatchService<ServerWatchEngine>,
}

impl WatchServiceWrapper {
    pub fn new(config: WatchConfig, state: AppState) -> Self {
        let engine = Arc::new(ServerWatchEngine::new(state));
        let inner = EngineWatchService::new(
            EngineConfig {
                watch_dir: config.watch_dir,
                auto_start: config.auto_start,
                enabled: config.enabled,
                max_depth: config.max_depth,
            },
            engine,
        );
        Self { inner }
    }

    pub fn config(&self) -> WatchConfig {
        let config = self.inner.config();
        WatchConfig {
            watch_dir: config.watch_dir,
            auto_start: config.auto_start,
            enabled: config.enabled,
            max_depth: config.max_depth,
        }
    }

    pub fn set_max_depth(&mut self, depth: u32) {
        self.inner.set_max_depth(depth);
    }

    pub fn set_auto_start(&mut self, enabled: bool) {
        self.inner.set_auto_start(enabled);
    }

    pub async fn start(&mut self) -> Result<(), String> {
        self.inner.start().await
    }

    pub async fn stop(&mut self) {
        self.inner.stop().await;
    }

    pub async fn get_status(&self) -> WatchStatus {
        self.inner.get_status().await.into()
    }

    pub async fn list_files(&self) -> Vec<WatchedFile> {
        self.inner.list_files().await.into_iter().map(Into::into).collect()
    }

    pub async fn reload_file(&self, filename: &str) -> Result<(), String> {
        self.inner.reload_file(filename).await
    }

    pub async fn reload_all(&self) -> Result<u32, String> {
        self.inner.reload_all().await
    }

    pub async fn delete_file(&self, filename: &str) -> Result<(), String> {
        self.inner.delete_file(filename).await
    }

    pub async fn remove_info_hash(&self, info_hash: &[u8; 20]) {
        self.inner.remove_info_hash(info_hash).await;
    }
}

pub type WatchService = WatchServiceWrapper;
