use rustatio_core::{
    FakerConfig, FakerState, GridImportSettings, InstanceSummary, PresetSettings, RatioFaker, TorrentInfo,
};
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::logging::log_and_emit;
use crate::state::{hex_info_hash, now_secs, AppState, FakerInstance};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GridActionResponse {
    succeeded: Vec<u32>,
    failed: Vec<GridActionError>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GridActionError {
    id: u32,
    error: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GridImportResponse {
    imported: Vec<GridImportedInstance>,
    errors: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GridImportedInstance {
    id: u32,
    name: String,
    info_hash: String,
}

async fn import_torrent_files(
    paths: Vec<std::path::PathBuf>,
    config: &GridImportSettings,
    state: &AppState,
    app: &AppHandle,
) -> GridImportResponse {
    let mut imported = Vec::new();
    let mut errors = Vec::new();
    let mut auto_start_ids = Vec::new();

    let now = now_secs();

    let next_id = state.next_instance_id.write().await;
    let base_id = *next_id;
    drop(next_id);

    let mut fakers_lock = state.fakers.write().await;

    for (i, path) in paths.iter().enumerate() {
        let torrent = match TorrentInfo::from_file(path) {
            Ok(t) => t,
            Err(e) => {
                errors.push(format!("{}: {}", path.display(), e));
                continue;
            }
        };

        let preset = config.resolve_for_instance();
        let faker_config: FakerConfig = preset.into();
        let torrent_name = torrent.name.clone();
        let torrent_info_hash = torrent.info_hash;

        let faker = match RatioFaker::new(torrent.clone(), faker_config.clone()) {
            Ok(f) => f,
            Err(e) => {
                errors.push(format!("{}: {}", torrent_name, e));
                continue;
            }
        };

        let instance_id = base_id + i as u32;
        let hash_hex = hex_info_hash(&torrent_info_hash);

        fakers_lock.insert(
            instance_id,
            FakerInstance {
                faker,
                torrent,
                config: faker_config,
                cumulative_uploaded: 0,
                cumulative_downloaded: 0,
                tags: config.tags.clone(),
                created_at: now,
            },
        );

        imported.push(GridImportedInstance {
            id: instance_id,
            name: torrent_name,
            info_hash: hash_hex,
        });

        if config.auto_start {
            auto_start_ids.push(instance_id);
        }
    }

    let max_id = imported.iter().map(|i| i.id).max().unwrap_or(base_id);
    drop(fakers_lock);
    let mut next_id = state.next_instance_id.write().await;
    if max_id >= *next_id {
        *next_id = max_id + 1;
    }
    drop(next_id);

    log_and_emit!(
        app,
        info,
        "Imported {} torrent(s) ({} errors)",
        imported.len(),
        errors.len()
    );

    if !auto_start_ids.is_empty() {
        let state_fakers = state.fakers.clone();
        let app_handle = app.clone();
        let stagger = config.stagger_start_secs;

        tauri::async_runtime::spawn(async move {
            for (i, id) in auto_start_ids.iter().enumerate() {
                if i > 0 {
                    if let Some(delay) = stagger {
                        if delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                        }
                    }
                }
                let mut fakers = state_fakers.write().await;
                if let Some(instance) = fakers.get_mut(id) {
                    match instance.faker.start().await {
                        Ok(_) => {
                            log_and_emit!(&app_handle, *id, info, "Auto-started");
                        }
                        Err(e) => {
                            log_and_emit!(&app_handle, *id, error, "Auto-start failed: {}", e);
                        }
                    }
                }
                drop(fakers);
            }
            log_and_emit!(
                &app_handle,
                info,
                "Auto-start complete for {} instance(s)",
                auto_start_ids.len()
            );
        });
    }

    GridImportResponse { imported, errors }
}

#[tauri::command]
pub async fn grid_import_folder(
    path: String,
    config: GridImportSettings,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<GridImportResponse, String> {
    log_and_emit!(&app, info, "Grid importing from folder: {}", path);

    let dir = std::path::Path::new(&path);
    if !dir.is_dir() {
        return Err(format!("Not a directory: {}", path));
    }

    let mut torrent_paths = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("torrent") {
            torrent_paths.push(p);
        }
    }

    if torrent_paths.is_empty() {
        return Ok(GridImportResponse {
            imported: vec![],
            errors: vec!["No .torrent files found in directory".to_string()],
        });
    }

    torrent_paths.sort();
    log_and_emit!(&app, info, "Found {} .torrent files", torrent_paths.len());

    let result = import_torrent_files(torrent_paths, &config, &state, &app).await;
    log_and_emit!(
        &app,
        info,
        "Grid import complete: {} imported, {} errors",
        result.imported.len(),
        result.errors.len()
    );

    state.save_state().await;
    Ok(result)
}

#[tauri::command]
pub async fn grid_import_files(
    paths: Vec<String>,
    config: GridImportSettings,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<GridImportResponse, String> {
    log_and_emit!(&app, info, "Grid importing {} files", paths.len());

    let torrent_paths: Vec<std::path::PathBuf> = paths.iter().map(std::path::PathBuf::from).collect();
    let result = import_torrent_files(torrent_paths, &config, &state, &app).await;
    log_and_emit!(
        &app,
        info,
        "Grid import complete: {} imported, {} errors",
        result.imported.len(),
        result.errors.len()
    );

    state.save_state().await;
    Ok(result)
}

#[tauri::command]
pub async fn grid_start(
    ids: Vec<u32>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<GridActionResponse, String> {
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    for id in ids {
        let mut fakers = state.fakers.write().await;
        match fakers.get_mut(&id) {
            Some(instance) => match instance.faker.start().await {
                Ok(_) => {
                    log_and_emit!(&app, id, info, "Started via grid action");
                    succeeded.push(id);
                }
                Err(e) => {
                    failed.push(GridActionError {
                        id,
                        error: e.to_string(),
                    });
                }
            },
            None => {
                failed.push(GridActionError {
                    id,
                    error: format!("Instance {} not found", id),
                });
            }
        }
        drop(fakers);
    }

    state.save_state().await;
    Ok(GridActionResponse { succeeded, failed })
}

#[tauri::command]
pub async fn grid_stop(
    ids: Vec<u32>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<GridActionResponse, String> {
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    for id in ids {
        let mut fakers = state.fakers.write().await;
        match fakers.get_mut(&id) {
            Some(instance) => {
                let final_stats = instance.faker.get_stats().await;
                match instance.faker.stop().await {
                    Ok(_) => {
                        instance.cumulative_uploaded = final_stats.uploaded;
                        instance.cumulative_downloaded = final_stats.downloaded;
                        log_and_emit!(&app, id, info, "Stopped via grid action");
                        succeeded.push(id);
                    }
                    Err(e) => {
                        failed.push(GridActionError {
                            id,
                            error: e.to_string(),
                        });
                    }
                }
            }
            None => {
                failed.push(GridActionError {
                    id,
                    error: format!("Instance {} not found", id),
                });
            }
        }
        drop(fakers);
    }

    state.save_state().await;
    Ok(GridActionResponse { succeeded, failed })
}

#[tauri::command]
pub async fn grid_pause(
    ids: Vec<u32>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<GridActionResponse, String> {
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    let mut fakers = state.fakers.write().await;
    for id in ids {
        match fakers.get_mut(&id) {
            Some(instance) => match instance.faker.pause().await {
                Ok(_) => {
                    log_and_emit!(&app, id, info, "Paused via grid action");
                    succeeded.push(id);
                }
                Err(e) => {
                    failed.push(GridActionError {
                        id,
                        error: e.to_string(),
                    });
                }
            },
            None => {
                failed.push(GridActionError {
                    id,
                    error: format!("Instance {} not found", id),
                });
            }
        }
    }

    drop(fakers);
    state.save_state().await;
    Ok(GridActionResponse { succeeded, failed })
}

#[tauri::command]
pub async fn grid_resume(
    ids: Vec<u32>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<GridActionResponse, String> {
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    let mut fakers = state.fakers.write().await;
    for id in ids {
        match fakers.get_mut(&id) {
            Some(instance) => match instance.faker.resume().await {
                Ok(_) => {
                    log_and_emit!(&app, id, info, "Resumed via grid action");
                    succeeded.push(id);
                }
                Err(e) => {
                    failed.push(GridActionError {
                        id,
                        error: e.to_string(),
                    });
                }
            },
            None => {
                failed.push(GridActionError {
                    id,
                    error: format!("Instance {} not found", id),
                });
            }
        }
    }

    drop(fakers);
    state.save_state().await;
    Ok(GridActionResponse { succeeded, failed })
}

#[tauri::command]
pub async fn grid_delete(
    ids: Vec<u32>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<GridActionResponse, String> {
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    let mut fakers = state.fakers.write().await;
    for id in ids {
        match fakers.remove(&id) {
            Some(mut instance) => {
                let _ = instance.faker.stop().await;
                log_and_emit!(&app, id, info, "Deleted via grid action");
                succeeded.push(id);
            }
            None => {
                failed.push(GridActionError {
                    id,
                    error: format!("Instance {} not found", id),
                });
            }
        }
    }

    drop(fakers);
    state.save_state().await;
    Ok(GridActionResponse { succeeded, failed })
}

#[tauri::command]
pub async fn grid_update_config(
    ids: Vec<u32>,
    config: PresetSettings,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<GridActionResponse, String> {
    let faker_config: FakerConfig = config.into();
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    let mut fakers = state.fakers.write().await;
    for id in ids {
        match fakers.get_mut(&id) {
            Some(instance) => {
                let mut new_config = faker_config.clone();
                new_config.initial_uploaded = instance.cumulative_uploaded;
                new_config.initial_downloaded = instance.cumulative_downloaded;
                let existing_stats = instance.faker.get_stats().await;
                new_config.completion_percent = existing_stats.torrent_completion;

                match RatioFaker::new(instance.torrent.clone(), new_config) {
                    Ok(faker) => {
                        instance.faker = faker;
                        instance.config = faker_config.clone();
                        log_and_emit!(&app, id, info, "Config updated via grid action");
                        succeeded.push(id);
                    }
                    Err(e) => {
                        failed.push(GridActionError {
                            id,
                            error: e.to_string(),
                        });
                    }
                }
            }
            None => {
                failed.push(GridActionError {
                    id,
                    error: format!("Instance {} not found", id),
                });
            }
        }
    }

    drop(fakers);
    state.save_state().await;
    Ok(GridActionResponse { succeeded, failed })
}

#[tauri::command]
pub async fn grid_tag(
    ids: Vec<u32>,
    add_tags: Vec<String>,
    remove_tags: Vec<String>,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let mut fakers = state.fakers.write().await;
    let mut updated = 0u32;

    for id in ids {
        if let Some(instance) = fakers.get_mut(&id) {
            for tag in &remove_tags {
                instance.tags.retain(|t| t != tag);
            }
            for tag in &add_tags {
                if !instance.tags.contains(tag) {
                    instance.tags.push(tag.clone());
                }
            }
            updated += 1;
        }
    }

    drop(fakers);
    state.save_state().await;
    Ok(updated)
}

#[tauri::command]
pub async fn set_instance_tags(instance_id: u32, tags: Vec<String>, state: State<'_, AppState>) -> Result<(), String> {
    let mut fakers = state.fakers.write().await;

    if let Some(instance) = fakers.get_mut(&instance_id) {
        instance.tags = tags;
        Ok(())
    } else {
        Err(format!("Instance {} not found", instance_id))
    }?;

    drop(fakers);
    state.save_state().await;
    Ok(())
}

#[tauri::command]
pub async fn list_summaries(state: State<'_, AppState>) -> Result<Vec<InstanceSummary>, String> {
    let fakers = state.fakers.read().await;
    let mut summaries = Vec::new();

    for (id, instance) in fakers.iter() {
        let stats = instance.faker.get_stats().await;
        let state_str = match stats.state {
            FakerState::Paused => "paused",
            _ if stats.is_idling => "idle",
            FakerState::Idle => "idle",
            FakerState::Starting => "starting",
            FakerState::Running => "running",
            FakerState::Stopping => "stopping",
            FakerState::Stopped => "stopped",
        };

        summaries.push(InstanceSummary {
            id: id.to_string(),
            name: instance.torrent.name.clone(),
            info_hash: hex_info_hash(&instance.torrent.info_hash),
            state: state_str.to_string(),
            tags: instance.tags.clone(),
            total_size: instance.torrent.total_size,
            uploaded: stats.uploaded,
            downloaded: stats.downloaded,
            ratio: stats.ratio,
            current_upload_rate: stats.current_upload_rate,
            current_download_rate: stats.current_download_rate,
            seeders: stats.seeders,
            leechers: stats.leechers,
            left: stats.left,
            torrent_completion: stats.torrent_completion,
            source: "desktop".to_string(),
            created_at: instance.created_at,
        });
    }

    summaries.sort_by_key(|s| s.id.parse::<u32>().unwrap_or(0));
    Ok(summaries)
}
