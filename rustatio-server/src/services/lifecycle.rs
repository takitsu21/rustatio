use super::state::AppState;
use async_trait::async_trait;
use rustatio_core::logger::set_instance_context_str;
use rustatio_core::FakerStats;

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

        let faker = {
            let instances = self.instances.read().await;
            let instance = instances.get(id).ok_or("Instance not found")?;
            instance.faker.clone()
        };

        faker.write().await.start().await.map_err(|e| e.to_string())?;

        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after starting instance: {}", e);
        }

        Ok(())
    }

    async fn stop_instance(&self, id: &str) -> Result<FakerStats, String> {
        set_instance_context_str(Some(id));

        let faker = {
            let instances = self.instances.read().await;
            let instance = instances.get(id).ok_or("Instance not found")?;
            instance.faker.clone()
        };

        let stats = faker.read().await.get_stats().await;
        faker.write().await.stop().await.map_err(|e| e.to_string())?;

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

        Ok(stats)
    }

    async fn pause_instance(&self, id: &str) -> Result<(), String> {
        set_instance_context_str(Some(id));

        let faker = {
            let instances = self.instances.read().await;
            let instance = instances.get(id).ok_or("Instance not found")?;
            instance.faker.clone()
        };

        faker.write().await.pause().await.map_err(|e| e.to_string())?;

        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after pausing instance: {}", e);
        }

        Ok(())
    }

    async fn resume_instance(&self, id: &str) -> Result<(), String> {
        set_instance_context_str(Some(id));

        let faker = {
            let instances = self.instances.read().await;
            let instance = instances.get(id).ok_or("Instance not found")?;
            instance.faker.clone()
        };

        faker.write().await.resume().await.map_err(|e| e.to_string())?;

        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after resuming instance: {}", e);
        }

        Ok(())
    }

    async fn update_instance(&self, id: &str) -> Result<FakerStats, String> {
        set_instance_context_str(Some(id));

        let faker = {
            let instances = self.instances.read().await;
            let instance = instances.get(id).ok_or("Instance not found")?;
            instance.faker.clone()
        };

        faker.write().await.update().await.map_err(|e| e.to_string())?;
        let stats = faker.read().await.get_stats().await;
        Ok(stats)
    }

    async fn update_stats_only(&self, id: &str) -> Result<FakerStats, String> {
        set_instance_context_str(Some(id));

        let faker = {
            let instances = self.instances.read().await;
            let instance = instances.get(id).ok_or("Instance not found")?;
            instance.faker.clone()
        };

        faker
            .write()
            .await
            .update_stats_only()
            .await
            .map_err(|e| e.to_string())?;
        let stats = faker.read().await.get_stats().await;
        Ok(stats)
    }
}
