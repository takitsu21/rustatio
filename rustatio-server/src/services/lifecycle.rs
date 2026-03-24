use super::state::AppState;
use async_trait::async_trait;
use rustatio_core::logger::set_instance_context_str;
use rustatio_core::FakerStats;
use std::sync::Arc;

#[async_trait]
pub trait InstanceLifecycle {
    async fn start_instance(&self, id: &str) -> Result<(), String>;
    async fn stop_instance(&self, id: &str) -> Result<FakerStats, String>;
    async fn pause_instance(&self, id: &str) -> Result<(), String>;
    async fn resume_instance(&self, id: &str) -> Result<(), String>;
    async fn update_instance(&self, id: &str) -> Result<FakerStats, String>;
    async fn update_stats_only(&self, id: &str) -> Result<FakerStats, String>;
}

#[async_trait]
impl InstanceLifecycle for AppState {
    async fn start_instance(&self, id: &str) -> Result<(), String> {
        set_instance_context_str(Some(id));

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

        Ok(())
    }

    async fn stop_instance(&self, id: &str) -> Result<FakerStats, String> {
        set_instance_context_str(Some(id));

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

        Ok(stats)
    }

    async fn pause_instance(&self, id: &str) -> Result<(), String> {
        set_instance_context_str(Some(id));

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

        Ok(())
    }

    async fn resume_instance(&self, id: &str) -> Result<(), String> {
        set_instance_context_str(Some(id));

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

        Ok(())
    }

    async fn update_instance(&self, id: &str) -> Result<FakerStats, String> {
        set_instance_context_str(Some(id));

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
    }

    async fn update_stats_only(&self, id: &str) -> Result<FakerStats, String> {
        set_instance_context_str(Some(id));

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
    }
}
