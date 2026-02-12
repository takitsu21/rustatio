use rustatio_core::validation;
use rustatio_core::{FakerConfig, FakerState, RatioFaker, TorrentInfo};
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
    let mut fakers = state.fakers.write().await;
    let instance = fakers
        .get_mut(&instance_id)
        .ok_or_else(|| format!("Instance {} not found", instance_id))?;

    let stats = instance.faker.get_stats().await;
    if matches!(
        stats.state,
        FakerState::Running | FakerState::Starting | FakerState::Paused
    ) {
        return Err("Cannot update config while faker is running".to_string());
    }

    instance.config = config;

    // Recreate the faker so stats reflect the new config
    let faker = RatioFaker::new(instance.torrent.clone(), instance.config.clone())
        .map_err(|e| format!("Failed to create faker: {}", e))?;
    instance.faker = faker;

    drop(fakers);
    state.save_state().await;
    Ok(())
}

#[tauri::command]
pub async fn delete_instance(instance_id: u32, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let mut fakers = state.fakers.write().await;

    if let Some(mut instance) = fakers.remove(&instance_id) {
        if let Err(e) = instance.faker.stop().await {
            log_and_emit!(&app, warn, "Error stopping faker on delete: {}", e);
        }
        log_and_emit!(&app, info, "Deleted instance {}", instance_id);
    } else {
        log::info!("Deleted instance {} (was not started)", instance_id);
    }

    drop(fakers);
    state.save_state().await;
    Ok(())
}

#[tauri::command]
pub async fn list_instances(state: State<'_, AppState>) -> Result<Vec<InstanceInfo>, String> {
    let fakers = state.fakers.read().await;

    let mut instances: Vec<InstanceInfo> = vec![];
    for (id, instance) in fakers.iter() {
        let stats = instance.faker.get_stats().await;
        instances.push(InstanceInfo {
            id: *id,
            torrent_name: Some(instance.torrent.name.clone()),
            is_running: matches!(
                stats.state,
                FakerState::Starting | FakerState::Running | FakerState::Stopping
            ),
            is_paused: matches!(stats.state, FakerState::Paused),
        });
    }

    instances.sort_by_key(|i| i.id);
    Ok(instances)
}

#[tauri::command]
pub async fn load_torrent(path: String, app: AppHandle) -> Result<TorrentInfo, String> {
    let validated_path = validation::validate_torrent_path(&path).map_err(|e| {
        let error_msg = format!("Invalid torrent path: {}", e);
        log_and_emit!(&app, error, "{}", error_msg);
        error_msg
    })?;

    log_and_emit!(&app, info, "Loading torrent from: {}", validated_path.display());

    match TorrentInfo::from_file(validated_path.to_str().unwrap_or(&path)) {
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
            let error_msg = format!("Failed to load torrent: {}", e);
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
        let error_msg = format!("Invalid torrent path: {}", e);
        log_and_emit!(&app, error, "{}", error_msg);
        error_msg
    })?;

    let torrent = TorrentInfo::from_file(validated_path.to_str().unwrap_or(&path)).map_err(|e| {
        let error_msg = format!("Failed to load torrent: {}", e);
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
    match fakers.entry(instance_id) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            let config = FakerConfig::default();
            let faker = RatioFaker::new(torrent.clone(), config.clone())
                .map_err(|e| format!("Failed to create faker: {}", e))?;

            let now = crate::state::now_secs();

            entry.insert(FakerInstance {
                faker,
                torrent: torrent.clone(),
                config,
                cumulative_uploaded: 0,
                cumulative_downloaded: 0,
                tags: vec![],
                created_at: now,
            });
        }
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            let instance = entry.get_mut();
            let config = instance.config.clone();
            let faker =
                RatioFaker::new(torrent.clone(), config).map_err(|e| format!("Failed to create faker: {}", e))?;
            instance.torrent = torrent.clone();
            instance.faker = faker;
        }
    }
    drop(fakers);

    state.save_state().await;
    Ok(torrent)
}

#[tauri::command]
pub async fn get_instance_torrent(instance_id: u32, state: State<'_, AppState>) -> Result<TorrentInfo, String> {
    let fakers = state.fakers.read().await;
    let instance = fakers
        .get(&instance_id)
        .ok_or_else(|| format!("Instance {} not found", instance_id))?;
    Ok(instance.torrent.clone())
}
