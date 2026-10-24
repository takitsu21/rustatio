use crate::state::{build_summary, AppState};
use rustatio_core::logger::set_instance_context_str;
use rustatio_core::{FakerState, FakerStats, InstanceSummary, PostStopAction, RatioFakerHandle};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const TICK: Duration = Duration::from_secs(5);

fn should_update(state: FakerState, tracker_error: Option<&str>, retry_due: bool) -> bool {
    match state {
        FakerState::Running => true,
        FakerState::Stopped if tracker_error == Some("Tracker unavailable") => retry_due,
        _ => false,
    }
}

fn should_delete(stats: &FakerStats) -> bool {
    stats.stop_condition_met && stats.post_stop_action == PostStopAction::DeleteInstance
}

pub fn spawn(state: AppState, app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut timer = tokio::time::interval(TICK);
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut due = HashMap::new();

        loop {
            timer.tick().await;
            tick(&state, &app, &mut due).await;
        }
    });
}

async fn tick(state: &AppState, app: &AppHandle, due: &mut HashMap<u32, Instant>) {
    let mut dirty = false;
    let mut updated = Vec::new();
    let mut deleted = Vec::new();
    let now = Instant::now();

    let instances = snapshot(state).await;
    let ids: HashSet<u32> = instances.iter().map(|(id, _, _, _, _)| *id).collect();

    for (id, faker, name, interval, retry) in instances {
        let interval = Duration::from_secs(interval.max(1));
        let next = *due.entry(id).or_insert(now + interval);
        if !retry && now < next {
            continue;
        }

        let Some((before, after)) = run(id, &faker, &name).await else {
            continue;
        };

        due.insert(id, now + interval);

        if should_delete(&after) {
            delete(state, id).await;
            due.remove(&id);
            deleted.push(id);
            dirty = true;
            continue;
        }

        store(state, id, &after).await;
        dirty |= changed(&before, &after);
        updated.push(id);
    }

    due.retain(|id, _| ids.contains(id));
    publish(state, app, dirty, &updated, &deleted).await;
}

async fn snapshot(state: &AppState) -> Vec<(u32, Arc<RatioFakerHandle>, String, u64, bool)> {
    let fakers = state.fakers.read().await;
    fakers
        .iter()
        .map(|(id, instance)| {
            let name = if instance.summary.name.is_empty() {
                id.to_string()
            } else {
                instance.summary.name.clone()
            };
            let retry = instance.faker.with_stats(|stats| {
                matches!(stats.state, FakerState::Stopped)
                    && stats.tracker_error.as_deref() == Some("Tracker unavailable")
            });
            (*id, Arc::clone(&instance.faker), name, instance.config.update_interval_seconds, retry)
        })
        .collect()
}

async fn run(id: u32, faker: &RatioFakerHandle, name: &str) -> Option<(FakerStats, FakerStats)> {
    let before = faker.stats_snapshot();
    let retry = retry_due(&before, faker).await;

    if !should_update(before.state, before.tracker_error.as_deref(), retry) {
        return None;
    }

    set_instance_context_str(Some(name));
    let result = if retry {
        faker.recover_tracker().await.map(|_| ()).map_err(|error| error.to_string())
    } else {
        faker.update().await.map_err(|error| error.to_string())
    };

    if let Err(error) = result {
        log::warn!("Scheduler update failed for instance {id}: {error}");
        set_instance_context_str(None);
        return None;
    }

    let after = faker.stats_snapshot();
    set_instance_context_str(None);
    Some((before, after))
}

async fn retry_due(stats: &FakerStats, faker: &RatioFakerHandle) -> bool {
    matches!(stats.state, FakerState::Stopped)
        && stats.tracker_error.as_deref() == Some("Tracker unavailable")
        && faker.tracker_retry_due_now().await
}

async fn delete(state: &AppState, id: u32) {
    let removed = state.fakers.write().await.remove(&id);
    let Some(instance) = removed else {
        return;
    };

    if matches!(instance.source, rustatio_watch::InstanceSource::WatchFolder) {
        remove_hash(state, instance.summary.info_hash).await;
    }

    if let Err(error) = instance.faker.stop().await {
        log::warn!("Scheduler failed to stop deleted instance {id}: {error}");
    }
}

async fn remove_hash(state: &AppState, hash: [u8; 20]) {
    let guard = state.watch.read().await;
    if let Some(watch) = guard.as_ref() {
        watch.remove_info_hash(&hash).await;
    }
}

async fn store(state: &AppState, id: u32, stats: &FakerStats) {
    let mut fakers = state.fakers.write().await;
    if let Some(instance) = fakers.get_mut(&id) {
        instance.cumulative_uploaded = stats.uploaded;
        instance.cumulative_downloaded = stats.downloaded;
        instance.config.completion_percent = stats.torrent_completion;
    }
}

fn changed(before: &FakerStats, after: &FakerStats) -> bool {
    std::mem::discriminant(&after.state) != std::mem::discriminant(&before.state)
        || after.stop_condition_met != before.stop_condition_met
        || after.is_idling != before.is_idling
        || after.tracker_error != before.tracker_error
        || after.tracker_retry_attempt != before.tracker_retry_attempt
        || after.tracker_retry_at_ms != before.tracker_retry_at_ms
}

async fn summaries(state: &AppState, ids: &[u32]) -> Vec<InstanceSummary> {
    let fakers = state.fakers.read().await;
    ids.iter()
        .filter_map(|id| {
            fakers.get(id).map(|instance| {
                instance.faker.with_stats(|stats| build_summary(*id, instance, stats))
            })
        })
        .collect()
}

async fn publish(state: &AppState, app: &AppHandle, dirty: bool, updated: &[u32], deleted: &[u32]) {
    if !updated.is_empty() {
        let list = summaries(state, updated).await;
        if let Err(error) = app.emit("instance-summaries", list) {
            log::warn!("Failed to emit instance summaries: {error}");
        }
    }

    for id in deleted {
        if let Err(error) = app.emit("instance-deleted", id.to_string()) {
            log::warn!("Failed to emit deleted instance {id}: {error}");
        }
    }

    if !deleted.is_empty() {
        state.refresh_peer_listener_port().await;
    }

    if dirty {
        if let Err(error) = state.save_state().await {
            log::warn!("Scheduler failed to save state: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_running_or_due_tracker_retries_are_updated() {
        assert!(should_update(FakerState::Running, None, false));
        assert!(should_update(FakerState::Stopped, Some("Tracker unavailable"), true));
        assert!(!should_update(FakerState::Stopped, Some("Tracker unavailable"), false));
        assert!(!should_update(FakerState::Stopped, Some("Other error"), true));
        assert!(!should_update(FakerState::Paused, None, false));
        assert!(!should_update(FakerState::Idle, None, false));
    }
}
