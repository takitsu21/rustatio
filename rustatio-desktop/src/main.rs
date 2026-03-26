#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod commands;
mod logging;
mod persistence;
mod state;
mod watch;
#[cfg(test)]
mod watch_tests;

use async_trait::async_trait;
use rustatio_core::{
    AppConfig, FakerState, PeerCatalog, PeerListenerService, PeerListenerStatus, PeerLookup,
    RatioFaker, RatioFakerHandle,
};
use rustatio_watch::InstanceSource;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager, RunEvent, WindowEvent};
use tokio::sync::RwLock;

use logging::log_and_emit;
use state::{AppState, FakerInstance};

#[derive(Clone)]
struct DesktopPeerLookup {
    fakers: Arc<RwLock<HashMap<u32, FakerInstance>>>,
}

#[async_trait]
impl PeerLookup for DesktopPeerLookup {
    async fn snapshot(&self) -> PeerCatalog {
        let guard = self.fakers.read().await;
        let mut out = PeerCatalog::new();
        for instance in guard.values() {
            out.entry(instance.torrent.info_hash)
                .and_modify(|entry| {
                    *entry = Err("duplicate active instances for info_hash".to_string());
                })
                .or_insert_with(|| Ok(Arc::clone(&instance.faker)));
        }
        out
    }
}

/// Synchronous save for the exit handler (tokio runtime may be winding down)
fn save_state_sync(state: &AppState) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        log::warn!("No tokio runtime available for exit save");
        return;
    };

    let result = tokio::task::block_in_place(|| {
        handle.block_on(async move {
            let persisted = state.build_persisted_state().await;
            persistence::save_state(&persisted)
        })
    });

    match result {
        Ok(()) => log::info!("Final state saved successfully"),
        Err(e) => log::error!("Exit save failed: {e}"),
    }
}

fn save_state_blocking(state: AppState) {
    let rt = tokio::runtime::Handle::current();
    let result = rt.block_on(async move { state.save_state().await });
    if let Err(e) = result {
        log::error!("Periodic save failed: {e}");
    }
}

async fn restore_saved_instances(
    app_handle: tauri::AppHandle,
    state: AppState,
    saved_instances: HashMap<u32, persistence::PersistedInstance>,
    watch_auto_start: bool,
) {
    let mut auto_start_ids: Vec<u32> = Vec::new();

    for (id, persisted) in &saved_instances {
        let mut config = persisted.config.clone();
        config.initial_uploaded = persisted.cumulative_uploaded;
        config.initial_downloaded = persisted.cumulative_downloaded;

        let summary = Arc::new(persisted.torrent.clone());
        let torrent = Arc::new(persisted.torrent.to_info());

        match RatioFaker::new(Arc::clone(&torrent), config, Some(state.http_client.clone())) {
            Ok(faker) => {
                let was_running =
                    matches!(persisted.state, FakerState::Starting | FakerState::Running);
                let source = if persisted.from_watch_folder {
                    InstanceSource::WatchFolder
                } else {
                    InstanceSource::Manual
                };

                state.fakers.write().await.insert(
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
                        source,
                    },
                );

                let _ = app_handle.emit("instance-restored", *id);

                if was_running && matches!(source, InstanceSource::WatchFolder) && watch_auto_start
                {
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
                log_and_emit!(&app_handle, error, "Failed to restore instance {}: {}", id, e);
            }
        }
    }

    if !saved_instances.is_empty() {
        log_and_emit!(
            &app_handle,
            info,
            "Restored {} instance(s), {} to auto-start",
            saved_instances.len(),
            auto_start_ids.len()
        );
    }

    for id in &auto_start_ids {
        let faker = {
            let fakers = state.fakers.read().await;
            match fakers.get(id) {
                Some(instance) => Arc::clone(&instance.faker),
                None => continue,
            }
        };

        match faker.start().await {
            Ok(()) => {
                log_and_emit!(&app_handle, *id, info, "Auto-started (was running before shutdown)");
            }
            Err(e) => {
                log_and_emit!(&app_handle, *id, error, "Auto-start failed: {}", e);
            }
        };
    }

    state.refresh_peer_listener_port().await;
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn show_exit_prompt(app: &tauri::AppHandle, prompt_open: &Arc<AtomicBool>) {
    if prompt_open.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
        return;
    }

    if let Some(window) = app.get_webview_window("main") {
        ensure_window_interactive(&window);
        let _ = window.emit("app-close-requested", ());
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    } else {
        prompt_open.store(false, Ordering::Relaxed);
    }
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
    if let tauri::tray::TrayIconEvent::Click {
        button: tauri::tray::MouseButton::Left,
        button_state: tauri::tray::MouseButtonState::Up,
        ..
    } = event
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
    let saved_instances_map = saved_state.instances;
    let saved_default_config = saved_state.default_config.clone();
    let saved_watch_settings = saved_state.watch_settings;

    // Keep references for exit handler
    let fakers_for_exit = Arc::new(RwLock::new(HashMap::new()));
    let next_id_for_exit = Arc::new(RwLock::new(next_id));

    let should_exit = Arc::new(AtomicBool::new(false));
    let close_prompt_open = Arc::new(AtomicBool::new(false));

    let app_state = AppState {
        fakers: Arc::clone(&fakers_for_exit),
        next_instance_id: Arc::clone(&next_id_for_exit),
        config: Arc::new(RwLock::new(config)),
        http_client: rustatio_core::reqwest::Client::new(),
        watch: Arc::new(RwLock::new(None)),
        default_config: Arc::new(RwLock::new(saved_default_config)),
        watch_settings: Arc::new(RwLock::new(saved_watch_settings)),
        should_exit: Arc::clone(&should_exit),
        close_prompt_open: Arc::clone(&close_prompt_open),
        peer_listener: Arc::new(RwLock::new(None)),
        peer_listener_status: Arc::new(RwLock::new(PeerListenerStatus::default())),
    };

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
            commands::get_network_status,
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
            commands::get_watch_status,
            commands::list_watch_files,
            commands::delete_watch_file,
            commands::reload_watch_file,
            commands::reload_all_watch_files,
            commands::get_watch_config,
            commands::set_watch_config,
            commands::get_default_config,
            commands::set_default_config,
            commands::clear_default_config,
            commands::close_to_tray,
            commands::quit_app,
            commands::cancel_close_prompt,
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
            let state_inner = state.inner().clone();
            let peer_lookup = DesktopPeerLookup { fakers: Arc::clone(&state_inner.fakers) };
            let peer_listener_store = Arc::clone(&state_inner.peer_listener_status);
            let peer_listener_rx = {
                let state_ref = &state_inner;
                tauri::async_runtime::block_on(async {
                    let mut peer_listener = PeerListenerService::new();
                    peer_listener.start(Arc::new(peer_lookup));
                    let status_rx = peer_listener.subscribe();
                    let peer_listener = Arc::new(tokio::sync::Mutex::new(peer_listener));
                    state_ref.attach_peer_listener(Arc::clone(&peer_listener)).await;
                    status_rx
                })
            };
            tauri::async_runtime::spawn(async move {
                let mut rx = peer_listener_rx;
                while rx.changed().await.is_ok() {
                    let status = rx.borrow().clone();
                    *peer_listener_store.write().await = status;
                }
            });

            let saved_instances = saved_instances_map;
            let watch_settings =
                state_inner.watch_settings.try_read().ok().and_then(|guard| guard.clone());
            let default_config = Arc::clone(&state_inner.default_config);
            let watch_state = Arc::clone(&state_inner.watch);
            let watch_auto_start = watch_settings.clone().unwrap_or_default().auto_start;

            tauri::async_runtime::spawn(async move {
                restore_saved_instances(
                    app_handle.clone(),
                    state_inner.clone(),
                    saved_instances,
                    watch_auto_start,
                )
                .await;

                let mut watch =
                    watch::build_watch_service(state_inner, default_config, watch_settings);
                if let Err(e) = watch.start().await {
                    log::error!("Failed to start watch service: {e}");
                }
                let mut watch_guard = watch_state.write().await;
                *watch_guard = Some(watch);
            });

            // Periodic auto-save every 30 seconds
            let state_for_save: tauri::State<'_, AppState> = app.state();
            let state_for_save = state_for_save.inner().clone();

            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;

                    let _ = tokio::task::spawn_blocking({
                        let state = state_for_save.clone();
                        move || save_state_blocking(state)
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
                    show_exit_prompt(window.app_handle(), &close_prompt_open);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(move |app_handle, event| {
        if matches!(event, RunEvent::Exit) {
            log::info!("Application exiting, saving final state...");
            let state = app_handle.state::<AppState>();
            save_state_sync(state.inner());
        }
    });
}
