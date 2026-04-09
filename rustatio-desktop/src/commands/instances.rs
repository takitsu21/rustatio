use rustatio_core::validation;
use rustatio_core::{FakerConfig, RatioFaker, RatioFakerHandle, TorrentInfo, TorrentSummary};
use rustatio_watch::InstanceSource;
use std::sync::Arc;
use tauri::{AppHandle, State};

use crate::logging::log_and_emit;
use crate::state::{AppState, FakerInstance, InstanceInfo};

#[tauri::command]
pub async fn create_instance(state: State<'_, AppState>, app: AppHandle) -> Result<u32, String> {
    let mut next_id = state.next_instance_id.write().await;
    let instance_id = *next_id;
    *next_id += 1;

    log_and_emit!(&app, info, "Created instance {}", instance_id);
    Ok(instance_id)
}

#[tauri::command]
pub async fn update_instance_config(
    instance_id: u32,
    config: FakerConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.apply_instance_config(instance_id, config).await
}

#[tauri::command]
pub async fn delete_instance(
    instance_id: u32,
    force: Option<bool>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let is_forced = force.unwrap_or(false);

    if let Some(instance) = state.fakers.read().await.get(&instance_id) {
        if matches!(instance.source, InstanceSource::WatchFolder) && !is_forced {
            return Err(
                "Cannot delete watch folder instance. Delete the torrent file from the watch folder instead."
                    .to_string(),
            );
        }
    }

    // Remove from HashMap first, then stop — HTTP happens after lock is released
    let removed = {
        let mut fakers = state.fakers.write().await;
        fakers.remove(&instance_id)
    };

    if let Some(instance) = removed {
        if is_forced && matches!(instance.source, InstanceSource::WatchFolder) {
            let info_hash = instance.summary.info_hash;
            let watch_guard = state.watch.read().await;
            if let Some(watch) = watch_guard.as_ref() {
                watch.remove_info_hash(&info_hash).await;
            }
        }

        if let Err(e) = instance.faker.stop().await {
            log_and_emit!(&app, warn, "Error stopping faker on delete: {}", e);
        }
        log_and_emit!(&app, info, "Deleted instance {}", instance_id);
    } else {
        log::info!("Deleted instance {instance_id} (was not started)");
    }

    state.refresh_peer_listener_port().await;

    Ok(())
}

#[tauri::command]
pub async fn list_instances(state: State<'_, AppState>) -> Result<Vec<InstanceInfo>, String> {
    let fakers = state.fakers.read().await;

    let mut instances: Vec<InstanceInfo> = vec![];
    for (id, instance) in fakers.iter() {
        let stats = instance.faker.stats_snapshot();
        instances.push(InstanceInfo {
            id: id.to_string(),
            torrent: (*instance.summary).clone(),
            config: instance.config.clone(),
            stats,
            created_at: instance.created_at,
            source: match instance.source {
                InstanceSource::Manual => "manual".to_string(),
                InstanceSource::WatchFolder => "watch_folder".to_string(),
            },
            tags: instance.tags.clone(),
        });
    }

    instances.sort_by_key(|i| i.id.parse::<u32>().unwrap_or(0));
    Ok(instances)
}

#[tauri::command]
pub async fn load_torrent(path: String, app: AppHandle) -> Result<TorrentInfo, String> {
    let validated_path = validation::validate_torrent_path(&path).map_err(|e| {
        let error_msg = format!("Invalid torrent path: {e}");
        log_and_emit!(&app, error, "{}", error_msg);
        error_msg
    })?;

    log_and_emit!(&app, info, "Loading torrent from: {}", validated_path.display());

    match TorrentInfo::from_file_summary(validated_path.to_str().unwrap_or(&path)) {
        Ok(torrent) => {
            log_and_emit!(
                &app,
                info,
                "Torrent loaded: {} ({} bytes)",
                torrent.name,
                torrent.total_size
            );
            Ok(torrent)
        }
        Err(e) => {
            let error_msg = format!("Failed to load torrent: {e}");
            log_and_emit!(&app, error, "{}", error_msg);
            Err(error_msg)
        }
    }
}

#[tauri::command]
pub async fn load_instance_torrent(
    instance_id: u32,
    path: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<TorrentInfo, String> {
    let validated_path = validation::validate_torrent_path(&path).map_err(|e| {
        let error_msg = format!("Invalid torrent path: {e}");
        log_and_emit!(&app, error, "{}", error_msg);
        error_msg
    })?;

    let torrent = TorrentInfo::from_file_summary(validated_path.to_str().unwrap_or(&path))
        .map_err(|e| {
            let error_msg = format!("Failed to load torrent: {e}");
            log_and_emit!(&app, error, "{}", error_msg);
            error_msg
        })?;

    log_and_emit!(
        &app,
        instance_id,
        info,
        "Loaded torrent: {} ({} bytes)",
        torrent.name,
        torrent.total_size
    );

    let mut fakers = state.fakers.write().await;
    let response_torrent = torrent.clone();
    let torrent_arc = Arc::new(torrent.without_files());
    let summary_arc = Arc::new(torrent_arc.summary());
    match fakers.entry(instance_id) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            let config = FakerConfig::default();
            let faker = RatioFaker::new(
                Arc::clone(&torrent_arc),
                config.clone(),
                Some(state.http_client.clone()),
            )
            .map_err(|e| format!("Failed to create faker: {e}"))?;

            let now = crate::state::now_secs();

            entry.insert(FakerInstance {
                faker: Arc::new(RatioFakerHandle::new(faker)),
                torrent: torrent_arc,
                summary: summary_arc,
                config,
                cumulative_uploaded: 0,
                cumulative_downloaded: 0,
                tags: vec![],
                created_at: now,
                source: InstanceSource::Manual,
            });
        }
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            let instance = entry.get_mut();
            let config = instance.config.clone();
            let faker =
                RatioFaker::new(Arc::clone(&torrent_arc), config, Some(state.http_client.clone()))
                    .map_err(|e| format!("Failed to create faker: {e}"))?;
            instance.torrent = torrent_arc;
            instance.summary = summary_arc;
            instance.faker = Arc::new(RatioFakerHandle::new(faker));
            instance.source = InstanceSource::Manual;
        }
    }
    Ok(response_torrent)
}

#[tauri::command]
pub async fn get_instance_torrent(
    instance_id: u32,
    state: State<'_, AppState>,
) -> Result<TorrentInfo, String> {
    let fakers = state.fakers.read().await;
    let instance =
        fakers.get(&instance_id).ok_or_else(|| format!("Instance {instance_id} not found"))?;
    Ok((*instance.torrent).clone())
}

#[tauri::command]
pub async fn get_instance_summary(
    instance_id: u32,
    state: State<'_, AppState>,
) -> Result<TorrentSummary, String> {
    let fakers = state.fakers.read().await;
    let instance =
        fakers.get(&instance_id).ok_or_else(|| format!("Instance {instance_id} not found"))?;
    Ok((*instance.summary).clone())
}
