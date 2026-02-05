use super::persistence::InstanceSource;
use rustatio_core::{FakerConfig, FakerStats, RatioFaker, TorrentInfo};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use utoipa::ToSchema;

pub struct FakerInstance {
    pub faker: Arc<RwLock<RatioFaker>>,
    pub torrent: TorrentInfo,
    pub config: FakerConfig,
    pub torrent_info_hash: [u8; 20],
    pub cumulative_uploaded: u64,
    pub cumulative_downloaded: u64,
    pub created_at: u64,
    pub source: InstanceSource,
    pub task_handle: Option<JoinHandle<()>>,
    pub shutdown_tx: Option<mpsc::Sender<()>>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InstanceInfo {
    pub id: String,
    #[schema(value_type = Object)]
    pub torrent: TorrentInfo,
    #[schema(value_type = Object)]
    pub config: FakerConfig,
    #[schema(value_type = Object)]
    pub stats: FakerStats,
    pub created_at: u64,
    pub source: InstanceSource,
}
