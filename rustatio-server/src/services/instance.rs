use super::persistence::InstanceSource;
use rustatio_core::{FakerConfig, FakerStats, RatioFakerHandle, TorrentInfo, TorrentSummary};
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

pub struct FakerInstance {
    pub faker: Arc<RatioFakerHandle>,
    pub torrent: Arc<TorrentInfo>,
    pub summary: Arc<TorrentSummary>,
    pub config: FakerConfig,
    pub torrent_info_hash: [u8; 20],
    pub cumulative_uploaded: u64,
    pub cumulative_downloaded: u64,
    pub created_at: u64,
    pub source: InstanceSource,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InstanceInfo {
    pub id: String,
    #[schema(value_type = Object)]
    pub torrent: Arc<TorrentSummary>,
    #[schema(value_type = Object)]
    pub config: FakerConfig,
    #[schema(value_type = Object)]
    pub stats: FakerStats,
    pub created_at: u64,
    pub source: InstanceSource,
    pub tags: Vec<String>,
}
