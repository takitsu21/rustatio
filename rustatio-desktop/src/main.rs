#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod commands;
mod logging;
mod persistence;
mod state;

use rustatio_core::{AppConfig, FakerState, RatioFaker};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::RwLock;

use logging::log_and_emit;
use state::{AppState, FakerInstance};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Rustatio v{} (Multi-Instance)", env!("CARGO_PKG_VERSION"));

    let config = AppConfig::load_or_default();

    let saved_state = persistence::load_state();
    let next_id = saved_state.next_instance_id.max(1);
    let saved_instances = saved_state.instances;

    let app_state = AppState {
        fakers: Arc::new(RwLock::new(HashMap::new())),
        next_instance_id: Arc::new(RwLock::new(next_id)),
        config: Arc::new(RwLock::new(config)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_os::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::create_instance,
            commands::delete_instance,
            commands::list_instances,
            commands::load_torrent,
            commands::load_instance_torrent,
            commands::get_instance_torrent,
            commands::update_instance_config,
            commands::get_config,
            commands::update_config,
            commands::start_faker,
            commands::stop_faker,
            commands::update_faker,
            commands::update_stats_only,
            commands::get_stats,
            commands::scrape_tracker,
            commands::pause_faker,
            commands::resume_faker,
            commands::get_client_types,
            commands::get_client_infos,
            commands::write_file,
            commands::grid_import_folder,
            commands::grid_import_files,
            commands::grid_start,
            commands::grid_stop,
            commands::grid_pause,
            commands::grid_resume,
            commands::grid_delete,
            commands::grid_update_config,
            commands::grid_tag,
            commands::set_instance_tags,
            commands::list_summaries,
        ])
        .setup(move |app| {
            rustatio_core::logger::init_logger(app.handle().clone());

            let app_handle = app.handle().clone();
            let state: tauri::State<'_, AppState> = app.state();

            let fakers_arc = state.fakers.clone();
            let restored_instances = saved_instances;

            tauri::async_runtime::spawn(async move {
                let mut auto_start_ids: Vec<u32> = Vec::new();

                for (id, persisted) in &restored_instances {
                    let mut config = persisted.config.clone();
                    config.initial_uploaded = persisted.cumulative_uploaded;
                    config.initial_downloaded = persisted.cumulative_downloaded;

                    match RatioFaker::new(persisted.torrent.clone(), config) {
                        Ok(faker) => {
                            let was_running = matches!(persisted.state, FakerState::Starting | FakerState::Running);

                            fakers_arc.write().await.insert(
                                *id,
                                FakerInstance {
                                    faker,
                                    torrent: persisted.torrent.clone(),
                                    config: persisted.config.clone(),
                                    cumulative_uploaded: persisted.cumulative_uploaded,
                                    cumulative_downloaded: persisted.cumulative_downloaded,
                                    tags: persisted.tags.clone(),
                                    created_at: persisted.created_at,
                                },
                            );

                            let _ = app_handle.emit("instance-restored", *id);

                            if was_running {
                                auto_start_ids.push(*id);
                            }

                            log_and_emit!(&app_handle, *id, info, "Restored instance: {}", persisted.torrent.name);
                        }
                        Err(e) => {
                            log_and_emit!(&app_handle, error, "Failed to restore instance {}: {}", id, e);
                        }
                    }
                }

                if !restored_instances.is_empty() {
                    log_and_emit!(
                        &app_handle,
                        info,
                        "Restored {} instance(s), {} to auto-start",
                        restored_instances.len(),
                        auto_start_ids.len()
                    );
                }

                for id in &auto_start_ids {
                    let mut fakers = fakers_arc.write().await;
                    if let Some(instance) = fakers.get_mut(id) {
                        match instance.faker.start().await {
                            Ok(_) => {
                                log_and_emit!(&app_handle, *id, info, "Auto-started (was running before shutdown)");
                            }
                            Err(e) => {
                                log_and_emit!(&app_handle, *id, error, "Auto-start failed: {}", e);
                            }
                        }
                    }
                }
            });

            // Periodic auto-save every 30 seconds
            let state_for_save: tauri::State<'_, AppState> = app.state();
            let fakers_for_save = state_for_save.fakers.clone();
            let next_id_for_save = state_for_save.next_instance_id.clone();

            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;

                    let fakers = fakers_for_save.read().await;
                    let next_id = *next_id_for_save.read().await;
                    let now = persistence::now_timestamp();

                    let mut instances = HashMap::new();
                    for (id, instance) in fakers.iter() {
                        let stats = instance.faker.get_stats().await;
                        let mut config = instance.config.clone();
                        config.completion_percent = stats.torrent_completion;
                        instances.insert(
                            *id,
                            persistence::PersistedInstance {
                                id: *id,
                                torrent: instance.torrent.clone(),
                                config,
                                cumulative_uploaded: stats.uploaded,
                                cumulative_downloaded: stats.downloaded,
                                state: stats.state,
                                created_at: instance.created_at,
                                updated_at: now,
                                tags: instance.tags.clone(),
                            },
                        );
                    }
                    drop(fakers);

                    let persisted = persistence::PersistedState {
                        instances,
                        next_instance_id: next_id,
                        version: 1,
                    };

                    if let Err(e) = persistence::save_state(&persisted) {
                        log::error!("Periodic save failed: {}", e);
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
