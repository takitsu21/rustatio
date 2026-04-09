use super::instance::FakerInstance;
use super::state::AppState;
use rustatio_core::logger::set_instance_context_str;
use rustatio_core::{FakerState, RatioFakerHandle};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

pub struct Scheduler {
    shutdown_tx: Option<mpsc::Sender<()>>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self { shutdown_tx: None, task_handle: None }
    }

    pub fn start(
        &mut self,
        state: AppState,
        instances: Arc<RwLock<HashMap<String, FakerInstance>>>,
    ) {
        if self.task_handle.is_some() {
            return;
        }

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        let handle = tokio::spawn(scheduler_loop(state, instances, shutdown_rx));

        self.shutdown_tx = Some(shutdown_tx);
        self.task_handle = Some(handle);

        tracing::info!("Centralized scheduler started");
    }

    pub async fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        if let Some(handle) = self.task_handle.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
        }
        tracing::info!("Centralized scheduler stopped");
    }
}

async fn scheduler_loop(
    state: AppState,
    instances: Arc<RwLock<HashMap<String, FakerInstance>>>,
    mut shutdown_rx: mpsc::Receiver<()>,
) {
    let update_interval = Duration::from_secs(5);
    let save_interval = Duration::from_secs(30);
    let mut last_save = std::time::Instant::now();

    tracing::info!("Scheduler loop started");

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                tracing::info!("Scheduler received shutdown signal");
                break;
            }
            () = tokio::time::sleep(update_interval) => {
                let dirty = update_all_running_instances(&instances).await;

                if dirty {
                    if let Err(e) = state.save_state().await {
                        tracing::warn!("Scheduler: failed to save state after runtime change: {}", e);
                    }
                }

                if last_save.elapsed() >= save_interval {
                    if let Err(e) = state.save_state().await {
                        tracing::warn!("Scheduler: failed to save state: {}", e);
                    }
                    last_save = std::time::Instant::now();
                }
            }
        }
    }

    tracing::info!("Scheduler loop stopped");
}

async fn update_all_running_instances(
    instances: &Arc<RwLock<HashMap<String, FakerInstance>>>,
) -> bool {
    // Collect running instance IDs and their faker handles
    let running: Vec<(String, Arc<RatioFakerHandle>)> = {
        let guard = instances.read().await;
        guard.iter().map(|(id, inst)| (id.clone(), Arc::clone(&inst.faker))).collect()
    };

    let mut dirty = false;

    for (id, faker) in running {
        let before = faker.stats_snapshot();
        if !matches!(before.state, FakerState::Running) {
            continue;
        }

        let label = {
            let guard = instances.read().await;
            guard
                .get(&id)
                .map(|instance| instance.summary.name.clone())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| id.clone())
        };
        set_instance_context_str(Some(&label));
        if let Err(e) = faker.update().await {
            tracing::warn!("Scheduler: update failed for instance {}: {}", id, e);
            continue;
        }

        let after = faker.stats_snapshot();
        {
            let mut guard = instances.write().await;
            if let Some(instance) = guard.get_mut(&id) {
                instance.cumulative_uploaded = after.uploaded;
                instance.cumulative_downloaded = after.downloaded;
                instance.config.completion_percent = after.torrent_completion;
            }
        }

        if std::mem::discriminant(&after.state) != std::mem::discriminant(&before.state)
            || after.stop_condition_met != before.stop_condition_met
            || after.is_idling != before.is_idling
        {
            dirty = true;
        }
    }

    dirty
}
