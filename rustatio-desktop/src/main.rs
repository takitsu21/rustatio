#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod commands;
mod logging;
mod persistence;
mod state;

use rustatio_core::{AppConfig, FakerState, RatioFaker, RatioFakerHandle};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager, RunEvent, WindowEvent};
use tokio::sync::RwLock;

use logging::log_and_emit;
use state::{AppState, FakerInstance};

/// Synchronous save for the exit handler (tokio runtime may be winding down)
fn save_state_sync(
    fakers: &Arc<RwLock<HashMap<u32, FakerInstance>>>,
    next_instance_id: &Arc<RwLock<u32>>,
) {
    let Some(fakers) = fakers.try_read().ok() else {
        log::warn!("Could not acquire fakers lock for exit save");
        return;
    };
    let Some(next_id) = next_instance_id.try_read().ok() else {
        log::warn!("Could not acquire next_id lock for exit save");
        return;
    };
    let now = persistence::now_timestamp();

    let mut instances = HashMap::new();
    for (id, instance) in fakers.iter() {
        let stats = instance.faker.stats_snapshot();
        let mut config = instance.config.clone();
        config.completion_percent = stats.torrent_completion;
        instances.insert(
            *id,
            persistence::PersistedInstance {
                id: *id,
                torrent: (*instance.summary).clone(),
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

    let persisted =
        persistence::PersistedState { instances, next_instance_id: *next_id, version: 1 };

    if let Err(e) = persistence::save_state(&persisted) {
        log::error!("Exit save failed: {e}");
    } else {
        log::info!("Final state saved successfully");
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn show_exit_prompt(
    app: &tauri::AppHandle,
    should_exit: &Arc<AtomicBool>,
    prompt_open: &Arc<AtomicBool>,
) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

    if prompt_open.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
        return;
    }

    let app_handle = app.clone();
    let should_exit = Arc::clone(should_exit);
    let prompt_open = Arc::clone(prompt_open);

    let dialog = app_handle
        .dialog()
        .message("Do you want to quit Rustatio or close it to the tray?")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Quit".to_string(),
            "Close to Tray".to_string(),
        ));

    let dialog = if let Some(window) = app_handle.get_webview_window("main") {
        dialog.parent(&window)
    } else {
        dialog
    };

    dialog.show(move |quit| {
        prompt_open.store(false, Ordering::Relaxed);
        if let Some(window) = app_handle.get_webview_window("main") {
            ensure_window_interactive(&window);
        }

        if quit {
            should_exit.store(true, Ordering::Relaxed);
            app_handle.exit(0);
        } else if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.hide();
        }
    });
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn toggle_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let is_visible = window.is_visible().unwrap_or(true);
    if is_visible {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.unminimize();
        ensure_window_interactive(&window);
        let _ = window.set_focus();
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn ensure_window_interactive(window: &tauri::WebviewWindow) {
    let _ = window.set_enabled(true);
    let _ = window.set_closable(true);
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn handle_tray_menu_event(app: &tauri::AppHandle, id: &str, should_exit: &AtomicBool) {
    match id {
        "tray-show" => toggle_main_window(app),
        "tray-quit" => {
            should_exit.store(true, Ordering::Relaxed);
            app.exit(0);
        }
        _ => {}
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn handle_tray_icon_event(app: &tauri::AppHandle, event: &tauri::tray::TrayIconEvent) {
    if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event
    {
        toggle_main_window(app);
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Rustatio v{} (Multi-Instance)", env!("CARGO_PKG_VERSION"));

    let config = AppConfig::load_or_default();

    let saved_state = persistence::load_state();
    let next_id = saved_state.next_instance_id.max(1);
    let saved_instances = saved_state.instances;

    // Keep references for exit handler
    let fakers_for_exit = Arc::new(RwLock::new(HashMap::new()));
    let next_id_for_exit = Arc::new(RwLock::new(next_id));

    let app_state = AppState {
        fakers: Arc::clone(&fakers_for_exit),
        next_instance_id: Arc::clone(&next_id_for_exit),
        config: Arc::new(RwLock::new(config)),
        http_client: rustatio_core::reqwest::Client::new(),
    };

    let should_exit = Arc::new(AtomicBool::new(false));
    let close_prompt_open = Arc::new(AtomicBool::new(false));
    let should_exit_for_tray = Arc::clone(&should_exit);

    let app = tauri::Builder::default()
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
            commands::get_instance_summary,
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
            commands::set_log_level,
            commands::detect_linux_package_type,
            commands::grid_import_folder,
            commands::grid_import_files,
            commands::grid_start,
            commands::grid_stop,
            commands::grid_pause,
            commands::grid_resume,
            commands::grid_delete,
            commands::grid_update_config,
            commands::bulk_update_configs,
            commands::grid_tag,
            commands::set_instance_tags,
            commands::list_summaries,
        ])
        .setup(move |app| {
            rustatio_core::logger::init_logger(app.handle().clone());

            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::TrayIconBuilder;

                let show_item =
                    MenuItem::with_id(app, "tray-show", "Show/Hide", true, None::<&str>)?;
                let quit_item = MenuItem::with_id(app, "tray-quit", "Quit", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_item, &quit_item])?;
                let menu_state = Arc::clone(&should_exit_for_tray);

                let tray_icon = TrayIconBuilder::with_id("main-tray")
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(move |app, event| {
                        handle_tray_menu_event(app, event.id().as_ref(), &menu_state);
                    })
                    .on_tray_icon_event(|tray, event| {
                        handle_tray_icon_event(tray.app_handle(), &event);
                    })
                    .icon(
                        app.default_window_icon()
                            .cloned()
                            .expect("default window icon is required for tray"),
                    )
                    .tooltip("Rustatio")
                    .build(app)?;

                let _ = tray_icon;
            }

            let app_handle = app.handle().clone();
            let state: tauri::State<'_, AppState> = app.state();

            let fakers_arc = Arc::clone(&state.fakers);
            let restored_instances = saved_instances;
            let http_client = state.http_client.clone();

            tauri::async_runtime::spawn(async move {
                let mut auto_start_ids: Vec<u32> = Vec::new();

                for (id, persisted) in &restored_instances {
                    let mut config = persisted.config.clone();
                    config.initial_uploaded = persisted.cumulative_uploaded;
                    config.initial_downloaded = persisted.cumulative_downloaded;

                    let summary = Arc::new(persisted.torrent.clone());
                    let torrent = Arc::new(persisted.torrent.to_info());

                    match RatioFaker::new(Arc::clone(&torrent), config, Some(http_client.clone())) {
                        Ok(faker) => {
                            let was_running = matches!(
                                persisted.state,
                                FakerState::Starting | FakerState::Running
                            );

                            fakers_arc.write().await.insert(
                                *id,
                                FakerInstance {
                                    faker: Arc::new(RatioFakerHandle::new(faker)),
                                    torrent,
                                    summary,
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

                            log_and_emit!(
                                &app_handle,
                                *id,
                                info,
                                "Restored instance: {}",
                                persisted.torrent.name
                            );
                        }
                        Err(e) => {
                            log_and_emit!(
                                &app_handle,
                                error,
                                "Failed to restore instance {}: {}",
                                id,
                                e
                            );
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
                    let faker = {
                        let fakers = fakers_arc.read().await;
                        match fakers.get(id) {
                            Some(instance) => Arc::clone(&instance.faker),
                            None => continue,
                        }
                    };

                    match faker.start().await {
                        Ok(()) => {
                            log_and_emit!(
                                &app_handle,
                                *id,
                                info,
                                "Auto-started (was running before shutdown)"
                            );
                        }
                        Err(e) => {
                            log_and_emit!(&app_handle, *id, error, "Auto-start failed: {}", e);
                        }
                    };
                }
            });

            // Periodic auto-save every 30 seconds
            let state_for_save: tauri::State<'_, AppState> = app.state();
            let fakers_for_save = Arc::clone(&state_for_save.fakers);
            let next_id_for_save = Arc::clone(&state_for_save.next_instance_id);

            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;

                    let fakers = fakers_for_save.read().await;
                    let next_id = *next_id_for_save.read().await;
                    let now = persistence::now_timestamp();

                    let mut instances = HashMap::new();
                    for (id, instance) in fakers.iter() {
                        let stats = instance.faker.stats_snapshot();
                        let mut config = instance.config.clone();
                        config.completion_percent = stats.torrent_completion;
                        instances.insert(
                            *id,
                            persistence::PersistedInstance {
                                id: *id,
                                torrent: (*instance.summary).clone(),
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

                    let _ = tokio::task::spawn_blocking(move || {
                        if let Err(e) = persistence::save_state(&persisted) {
                            log::error!("Periodic save failed: {e}");
                        }
                    })
                    .await;
                }
            });

            Ok(())
        })
        .on_window_event({
            let should_exit = Arc::clone(&should_exit);
            let close_prompt_open = Arc::clone(&close_prompt_open);
            move |window, event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    if should_exit.load(Ordering::Relaxed) {
                        return;
                    }

                    api.prevent_close();
                    show_exit_prompt(window.app_handle(), &should_exit, &close_prompt_open);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(move |_app_handle, event| {
        if matches!(event, RunEvent::Exit) {
            log::info!("Application exiting, saving final state...");
            save_state_sync(&fakers_for_exit, &next_id_for_exit);
        }
    });
}
