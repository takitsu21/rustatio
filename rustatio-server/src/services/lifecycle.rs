use super::state::AppState;
use async_trait::async_trait;
use rustatio_core::FakerStats;
use std::sync::Arc;

fn resolve_instance_label(
    instances: &std::collections::HashMap<String, super::instance::FakerInstance>,
    id: &str,
) -> String {
    instances
        .get(id)
        .map(|instance| instance.summary.name.clone())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| id.to_string())
}

#[async_trait]
pub trait InstanceLifecycle {
    async fn start_instance(&self, id: &str) -> Result<(), String>;
    async fn recover_tracker_instance(&self, id: &str) -> Result<FakerStats, String>;
    async fn stop_instance(&self, id: &str) -> Result<FakerStats, String>;
    async fn pause_instance(&self, id: &str) -> Result<(), String>;
    async fn resume_instance(&self, id: &str) -> Result<(), String>;
    async fn update_instance(&self, id: &str) -> Result<FakerStats, String>;
    async fn update_stats_only(&self, id: &str) -> Result<FakerStats, String>;
}

impl AppState {
    async fn in_instance_ctx<F: std::future::Future>(&self, id: &str, fut: F) -> F::Output {
        let label = {
            let instances = self.instances.read().await;
            resolve_instance_label(&instances, id)
        };
        rustatio_core::logger::with_instance_context(label, fut).await
    }
}

#[async_trait]
impl InstanceLifecycle for AppState {
    async fn start_instance(&self, id: &str) -> Result<(), String> {
        self.in_instance_ctx(id, async {
            let (faker, restore) = {
                let instances = self.instances.read().await;
                let instance = instances.get(id).ok_or("Instance not found")?;
                let stats = instance.faker.stats_snapshot();
                let restore = matches!(
                    stats.state,
                    rustatio_core::FakerState::Running | rustatio_core::FakerState::Starting
                ) && stats.elapsed_time.as_secs() > 0;
                (Arc::clone(&instance.faker), restore)
            };

            if restore {
                faker.restore_running().await.map_err(|e| e.to_string())?;
            } else {
                faker.start().await.map_err(|e| e.to_string())?;
            }
            if let Err(e) = self.save_state().await {
                tracing::warn!("Failed to save state after starting instance: {}", e);
            }

            self.refresh_peer_listener_port().await;
            self.emit_instance_summaries(&[id.to_string()]).await;

            Ok(())
        })
        .await
    }

    async fn recover_tracker_instance(&self, id: &str) -> Result<FakerStats, String> {
        self.in_instance_ctx(id, async {
            let faker = {
                let instances = self.instances.read().await;
                let instance = instances.get(id).ok_or("Instance not found")?;
                Arc::clone(&instance.faker)
            };

            let stats = faker.recover_tracker().await.map_err(|e| e.to_string())?;

            {
                let mut instances = self.instances.write().await;
                if let Some(instance) = instances.get_mut(id) {
                    instance.cumulative_uploaded = stats.uploaded;
                    instance.cumulative_downloaded = stats.downloaded;
                    instance.config.completion_percent = stats.torrent_completion;
                }
            }

            if let Err(e) = self.save_state().await {
                tracing::warn!("Failed to save state after tracker recovery attempt: {}", e);
            }

            self.refresh_peer_listener_port().await;
            self.emit_instance_summaries(&[id.to_string()]).await;

            Ok(stats)
        })
        .await
    }

    async fn stop_instance(&self, id: &str) -> Result<FakerStats, String> {
        self.in_instance_ctx(id, async {
            let faker = {
                let instances = self.instances.read().await;
                let instance = instances.get(id).ok_or("Instance not found")?;
                Arc::clone(&instance.faker)
            };

            faker.stop().await.map_err(|e| e.to_string())?;
            let stats = faker.stats_snapshot();

            {
                let mut instances = self.instances.write().await;
                if let Some(instance) = instances.get_mut(id) {
                    instance.cumulative_uploaded = stats.uploaded;
                    instance.cumulative_downloaded = stats.downloaded;
                    instance.config.completion_percent = stats.torrent_completion;
                }
            }

            if let Err(e) = self.save_state().await {
                tracing::warn!("Failed to save state after stopping instance: {}", e);
            }

            self.refresh_peer_listener_port().await;
            self.emit_instance_summaries(&[id.to_string()]).await;

            Ok(stats)
        })
        .await
    }

    async fn pause_instance(&self, id: &str) -> Result<(), String> {
        self.in_instance_ctx(id, async {
            let faker = {
                let instances = self.instances.read().await;
                let instance = instances.get(id).ok_or("Instance not found")?;
                Arc::clone(&instance.faker)
            };

            faker.pause().await.map_err(|e| e.to_string())?;
            if let Err(e) = self.save_state().await {
                tracing::warn!("Failed to save state after pausing instance: {}", e);
            }

            self.refresh_peer_listener_port().await;
            self.emit_instance_summaries(&[id.to_string()]).await;

            Ok(())
        })
        .await
    }

    async fn resume_instance(&self, id: &str) -> Result<(), String> {
        self.in_instance_ctx(id, async {
            let faker = {
                let instances = self.instances.read().await;
                let instance = instances.get(id).ok_or("Instance not found")?;
                Arc::clone(&instance.faker)
            };

            faker.resume().await.map_err(|e| e.to_string())?;
            if let Err(e) = self.save_state().await {
                tracing::warn!("Failed to save state after resuming instance: {}", e);
            }

            self.refresh_peer_listener_port().await;
            self.emit_instance_summaries(&[id.to_string()]).await;

            Ok(())
        })
        .await
    }

    async fn update_instance(&self, id: &str) -> Result<FakerStats, String> {
        self.in_instance_ctx(id, async {
            let faker = {
                let instances = self.instances.read().await;
                let instance = instances.get(id).ok_or("Instance not found")?;
                Arc::clone(&instance.faker)
            };

            faker.update().await.map_err(|e| e.to_string())?;
            let stats = faker.stats_snapshot();

            {
                let mut instances = self.instances.write().await;
                if let Some(instance) = instances.get_mut(id) {
                    instance.cumulative_uploaded = stats.uploaded;
                    instance.cumulative_downloaded = stats.downloaded;
                    instance.config.completion_percent = stats.torrent_completion;
                }
            }

            Ok(stats)
        })
        .await
    }

    async fn update_stats_only(&self, id: &str) -> Result<FakerStats, String> {
        self.in_instance_ctx(id, async {
            let faker = {
                let instances = self.instances.read().await;
                let instance = instances.get(id).ok_or("Instance not found")?;
                Arc::clone(&instance.faker)
            };

            faker.update_stats_only().await.map_err(|e| e.to_string())?;
            let stats = faker.stats_snapshot();

            {
                let mut instances = self.instances.write().await;
                if let Some(instance) = instances.get_mut(id) {
                    instance.cumulative_uploaded = stats.uploaded;
                    instance.cumulative_downloaded = stats.downloaded;
                    instance.config.completion_percent = stats.torrent_completion;
                }
            }

            Ok(stats)
        })
        .await
    }
}
