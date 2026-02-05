use super::instance::FakerInstance;
use super::state::AppState;
use async_trait::async_trait;
use rustatio_core::logger::set_instance_context_str;
use rustatio_core::{FakerState, FakerStats, RatioFaker};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

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
            let mut instances = self.instances.write().await;
            let instance = instances.get_mut(id).ok_or("Instance not found")?;

            if let Some(tx) = instance.shutdown_tx.take() {
                let _ = tx.send(()).await;
            }
            if let Some(handle) = instance.task_handle.take() {
                handle.abort();
            }

            instance.faker.clone()
        };

        faker.write().await.start().await.map_err(|e| e.to_string())?;

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        let id_clone = id.to_string();
        let faker_clone = faker.clone();
        let instances_clone = self.instances.clone();
        let state_clone = self.clone();

        let task_handle = tokio::spawn(async move {
            background_update_loop(id_clone, faker_clone, instances_clone, state_clone, shutdown_rx).await;
        });

        {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(id) {
                instance.task_handle = Some(task_handle);
                instance.shutdown_tx = Some(shutdown_tx);
            }
        }

        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after starting instance: {}", e);
        }

        Ok(())
    }

    async fn stop_instance(&self, id: &str) -> Result<FakerStats, String> {
        set_instance_context_str(Some(id));

        let (faker, shutdown_tx, task_handle) = {
            let mut instances = self.instances.write().await;
            let instance = instances.get_mut(id).ok_or("Instance not found")?;
            (
                instance.faker.clone(),
                instance.shutdown_tx.take(),
                instance.task_handle.take(),
            )
        };

        stop_background_task(shutdown_tx, task_handle).await;

        let stats = faker.read().await.get_stats().await;
        faker.write().await.stop().await.map_err(|e| e.to_string())?;

        {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(id) {
                instance.cumulative_uploaded = stats.uploaded;
                instance.cumulative_downloaded = stats.downloaded;
            }
        }

        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after stopping instance: {}", e);
        }

        Ok(stats)
    }

    async fn pause_instance(&self, id: &str) -> Result<(), String> {
        set_instance_context_str(Some(id));

        let (faker, shutdown_tx, task_handle) = {
            let mut instances = self.instances.write().await;
            let instance = instances.get_mut(id).ok_or("Instance not found")?;
            (
                instance.faker.clone(),
                instance.shutdown_tx.take(),
                instance.task_handle.take(),
            )
        };

        stop_background_task(shutdown_tx, task_handle).await;

        faker.write().await.pause().await.map_err(|e| e.to_string())?;

        if let Err(e) = self.save_state().await {
            tracing::warn!("Failed to save state after pausing instance: {}", e);
        }

        Ok(())
    }

    async fn resume_instance(&self, id: &str) -> Result<(), String> {
        set_instance_context_str(Some(id));

        let faker = {
            let mut instances = self.instances.write().await;
            let instance = instances.get_mut(id).ok_or("Instance not found")?;

            if let Some(tx) = instance.shutdown_tx.take() {
                let _ = tx.send(()).await;
            }
            if let Some(handle) = instance.task_handle.take() {
                handle.abort();
            }

            instance.faker.clone()
        };

        faker.write().await.resume().await.map_err(|e| e.to_string())?;

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        let id_clone = id.to_string();
        let faker_clone = faker.clone();
        let instances_clone = self.instances.clone();
        let state_clone = self.clone();

        let task_handle = tokio::spawn(async move {
            background_update_loop(id_clone, faker_clone, instances_clone, state_clone, shutdown_rx).await;
        });

        {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(id) {
                instance.task_handle = Some(task_handle);
                instance.shutdown_tx = Some(shutdown_tx);
            }
        }

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

async fn stop_background_task(shutdown_tx: Option<mpsc::Sender<()>>, task_handle: Option<tokio::task::JoinHandle<()>>) {
    if let Some(tx) = shutdown_tx {
        let _ = tx.send(()).await;
    }
    if let Some(handle) = task_handle {
        let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
    }
}

async fn background_update_loop(
    id: String,
    faker: Arc<RwLock<RatioFaker>>,
    instances: Arc<RwLock<HashMap<String, FakerInstance>>>,
    state: AppState,
    mut shutdown_rx: mpsc::Receiver<()>,
) {
    let update_interval = Duration::from_secs(5);
    let save_interval = Duration::from_secs(30);
    let mut last_save = std::time::Instant::now();

    tracing::info!("Background update loop started for instance {}", id);

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                tracing::info!("Background update loop received shutdown signal for instance {}", id);
                break;
            }
            _ = tokio::time::sleep(update_interval) => {
                let should_continue = {
                    let instances_guard = instances.read().await;
                    if let Some(instance) = instances_guard.get(&id) {
                        let stats = instance.faker.read().await.get_stats().await;
                        matches!(stats.state, FakerState::Running)
                    } else {
                        false
                    }
                };

                if !should_continue {
                    tracing::info!("Instance {} no longer running, stopping background loop", id);
                    break;
                }

                set_instance_context_str(Some(&id));
                if let Err(e) = faker.write().await.update().await {
                    tracing::warn!("Background update failed for instance {}: {}", id, e);
                }

                if last_save.elapsed() >= save_interval {
                    if let Err(e) = state.save_state().await {
                        tracing::warn!("Failed to save state in background loop: {}", e);
                    }
                    last_save = std::time::Instant::now();
                }
            }
        }
    }

    tracing::info!("Background update loop stopped for instance {}", id);
}
