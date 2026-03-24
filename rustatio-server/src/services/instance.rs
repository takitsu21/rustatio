use super::persistence::InstanceSource;
use async_trait::async_trait;
use rustatio_core::{
    FakerConfig, FakerStats, PeerCatalog, PeerLookup, RatioFakerHandle, TorrentInfo, TorrentSummary,
};
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

#[derive(Clone)]
pub struct ServerPeerLookup {
    pub instances: Arc<tokio::sync::RwLock<std::collections::HashMap<String, FakerInstance>>>,
}

#[async_trait]
impl PeerLookup for ServerPeerLookup {
    async fn snapshot(&self) -> PeerCatalog {
        let guard = self.instances.read().await;
        let mut out = PeerCatalog::new();
        for instance in guard.values() {
            out.entry(instance.torrent_info_hash)
                .and_modify(|entry| {
                    *entry = Err("duplicate active instances for info_hash".to_string());
                })
                .or_insert_with(|| Ok(Arc::clone(&instance.faker)));
        }
        out
    }
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
