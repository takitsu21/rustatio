use super::instance::FakerInstance;
use super::state::AppState;
use rustatio_core::logger::set_instance_context_str;
use rustatio_core::FakerState;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

pub struct Scheduler {
    shutdown_tx: Option<mpsc::Sender<()>>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            shutdown_tx: None,
            task_handle: None,
        }
    }

    pub fn start(&mut self, state: AppState, instances: Arc<RwLock<HashMap<String, FakerInstance>>>) {
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
            _ = tokio::time::sleep(update_interval) => {
                update_all_running_instances(&instances).await;

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

async fn update_all_running_instances(instances: &Arc<RwLock<HashMap<String, FakerInstance>>>) {
    // Collect running instance IDs and their faker handles
    let running: Vec<(String, Arc<RwLock<rustatio_core::RatioFaker>>)> = {
        let guard = instances.read().await;
        guard
            .iter()
            .map(|(id, inst)| (id.clone(), inst.faker.clone()))
            .collect()
    };

    for (id, faker) in running {
        // Use try_write to skip instances currently locked by user operations (start/stop/pause)
        // This prevents the scheduler from blocking for up to 30s on a tracker announce
        let mut guard = match faker.try_write() {
            Ok(guard) => guard,
            Err(_) => {
                tracing::trace!("Scheduler: skipping instance {} (locked by another operation)", id);
                continue;
            }
        };

        let stats = guard.get_stats().await;
        if !matches!(stats.state, FakerState::Running) {
            continue;
        }

        set_instance_context_str(Some(&id));
        if let Err(e) = guard.update().await {
            tracing::warn!("Scheduler: update failed for instance {}: {}", id, e);
        }
    }
}
