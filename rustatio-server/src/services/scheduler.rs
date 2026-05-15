use super::instance::FakerInstance;
use super::lifecycle::InstanceLifecycle;
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
                let dirty = update_instances(&state, &instances).await;

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

async fn update_instances(
    state: &AppState,
    instances: &Arc<RwLock<HashMap<String, FakerInstance>>>,
) -> bool {
    let items: Vec<(String, Arc<RatioFakerHandle>)> = {
        let guard = instances.read().await;
        guard.iter().map(|(id, inst)| (id.clone(), Arc::clone(&inst.faker))).collect()
    };

    let mut dirty = false;

    for (id, faker) in items {
        let before = faker.stats_snapshot();
        let should_update = matches!(before.state, FakerState::Running);
        let should_retry = matches!(before.state, FakerState::Stopped)
            && before.tracker_error.as_deref() == Some("Tracker unavailable")
            && faker.tracker_retry_due_now().await;

        if !should_update && !should_retry {
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
        let result = if should_retry {
            state.recover_tracker_instance(&id).await.map(|_| ())
        } else {
            faker.update().await.map_err(|e| e.to_string())
        };
        if let Err(e) = result {
            let action = if should_retry { "tracker recovery" } else { "update" };
            tracing::warn!("Scheduler: {} failed for instance {}: {}", action, id, e);
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
            || after.tracker_error != before.tracker_error
            || after.tracker_retry_attempt != before.tracker_retry_attempt
            || after.tracker_retry_at_ms != before.tracker_retry_at_ms
        {
            dirty = true;
        }
    }

    dirty
}
