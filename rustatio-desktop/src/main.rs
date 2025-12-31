#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use rustatio_core::validation;
use rustatio_core::{AppConfig, FakerConfig, FakerState, FakerStats, RatioFaker, TorrentInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;

// Log event payload
#[derive(Clone, Serialize)]
struct LogEvent {
    timestamp: u64,
    level: String,
    message: String,
}

// Helper function to emit logs to frontend
fn emit_log(app: &AppHandle, level: &str, message: String) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_millis() as u64;

    let log_event = LogEvent {
        timestamp,
        level: level.to_string(),
        message,
    };

    let _ = app.emit("log-event", log_event);
}

// Macro to log and emit at the same time (with optional instance ID)
macro_rules! log_and_emit {
    ($app:expr, info, $($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            log::info!("{}", msg);
            emit_log($app, "info", msg);
        }
    };
    ($app:expr, warn, $($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            log::warn!("{}", msg);
            emit_log($app, "warn", msg);
        }
    };
    ($app:expr, $instance_id:expr, info, $($arg:tt)*) => {
        {
            let msg = format!("[Instance {}] {}", $instance_id, format!($($arg)*));
            log::info!("{}", msg);
            emit_log($app, "info", msg);
        }
    };
    ($app:expr, $instance_id:expr, warn, $($arg:tt)*) => {
        {
            let msg = format!("[Instance {}] {}", $instance_id, format!($($arg)*));
            log::warn!("{}", msg);
            emit_log($app, "warn", msg);
        }
    };
    ($app:expr, error, $($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            log::error!("{}", msg);
            emit_log($app, "error", msg);
        }
    };
    ($app:expr, $instance_id:expr, error, $($arg:tt)*) => {
        {
            let msg = format!("[Instance {}] {}", $instance_id, format!($($arg)*));
            log::error!("{}", msg);
            emit_log($app, "error", msg);
        }
    };
    ($app:expr, debug, $($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            log::debug!("{}", msg);
            emit_log($app, "debug", msg);
        }
    };
}

// Instance data
struct FakerInstance {
    faker: RatioFaker,
    torrent_name: String,
    // Cumulative stats across all sessions for this instance
    cumulative_uploaded: u64,
    cumulative_downloaded: u64,
}

// Instance info for frontend
#[derive(Clone, Serialize, Deserialize)]
struct InstanceInfo {
    id: u32,
    torrent_name: Option<String>,
    is_running: bool,
    is_paused: bool,
}

// Application state
struct AppState {
    fakers: Arc<RwLock<HashMap<u32, FakerInstance>>>,
    next_instance_id: Arc<RwLock<u32>>,
    config: Arc<RwLock<AppConfig>>,
}

// Tauri command: Create a new instance
#[tauri::command]
async fn create_instance(state: State<'_, AppState>, app: AppHandle) -> Result<u32, String> {
    let mut next_id = state.next_instance_id.write().await;
    let instance_id = *next_id;
    *next_id += 1;

    log_and_emit!(&app, info, "Created instance {}", instance_id);
    Ok(instance_id)
}

// Tauri command: Delete an instance
#[tauri::command]
async fn delete_instance(instance_id: u32, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let mut fakers = state.fakers.write().await;

    if let Some(mut instance) = fakers.remove(&instance_id) {
        // Stop the faker if it's running
        if let Err(e) = instance.faker.stop().await {
            log_and_emit!(&app, warn, "Error stopping faker on delete: {}", e);
        }
        log_and_emit!(&app, info, "Deleted instance {}", instance_id);
    } else {
        // Instance not in HashMap yet (never started) - this is okay
        log::info!("Deleted instance {} (was not started)", instance_id);
    }

    Ok(())
}

// Tauri command: List all instances
#[tauri::command]
async fn list_instances(state: State<'_, AppState>) -> Result<Vec<InstanceInfo>, String> {
    let fakers = state.fakers.read().await;

    let mut instances: Vec<InstanceInfo> = vec![];
    for (id, instance) in fakers.iter() {
        let stats = instance.faker.get_stats().await;
        instances.push(InstanceInfo {
            id: *id,
            torrent_name: Some(instance.torrent_name.clone()),
            is_running: matches!(stats.state, FakerState::Running | FakerState::Completed),
            is_paused: matches!(stats.state, FakerState::Paused),
        });
    }

    // Sort by ID
    instances.sort_by_key(|i| i.id);

    Ok(instances)
}

// Tauri command: Load a torrent file
#[tauri::command]
async fn load_torrent(path: String, app: AppHandle) -> Result<TorrentInfo, String> {
    // Validate the torrent file path
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

// Tauri command: Get app configuration
#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let config = state.config.read().await;
    Ok(config.clone())
}

// Tauri command: Update app configuration
#[tauri::command]
async fn update_config(config: AppConfig, state: State<'_, AppState>) -> Result<(), String> {
    // Validate configuration values
    validation::validate_rate(config.faker.default_upload_rate, "upload_rate").map_err(|e| format!("{}", e))?;
    validation::validate_rate(config.faker.default_download_rate, "download_rate").map_err(|e| format!("{}", e))?;
    validation::validate_update_interval(config.faker.update_interval).map_err(|e| format!("{}", e))?;
    validation::validate_port(config.client.default_port).map_err(|e| format!("{}", e))?;

    let mut app_config = state.config.write().await;
    *app_config = config.clone();

    // Save to file
    let path = AppConfig::default_path();
    config
        .save(&path)
        .map_err(|e| format!("Failed to save config: {}", e))?;

    log::info!("Configuration updated and saved");
    Ok(())
}

// Tauri command: Start ratio faking for an instance
#[tauri::command]
async fn start_faker(
    instance_id: u32,
    torrent: TorrentInfo,
    config: FakerConfig,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    // Validate faker configuration
    validation::validate_rate(config.upload_rate, "upload_rate").map_err(|e| format!("{}", e))?;
    validation::validate_rate(config.download_rate, "download_rate").map_err(|e| format!("{}", e))?;
    validation::validate_port(config.port).map_err(|e| format!("{}", e))?;
    validation::validate_percentage(config.completion_percent, "completion_percent").map_err(|e| format!("{}", e))?;

    if config.randomize_rates {
        validation::validate_percentage(config.random_range_percent, "random_range_percent")
            .map_err(|e| format!("{}", e))?;
    }

    log_and_emit!(&app, instance_id, info, "Starting faker for torrent: {}", torrent.name);
    log_and_emit!(
        &app,
        instance_id,
        info,
        "Upload: {} KB/s, Download: {} KB/s",
        config.upload_rate,
        config.download_rate
    );

    let torrent_name = torrent.name.clone();

    // Set instance context for logging
    rustatio_core::logger::set_instance_context(Some(instance_id));

    // Check if instance already exists (restarting) - use cumulative stats as initial values
    let mut config_with_cumulative = config.clone();
    let fakers = state.fakers.read().await;
    if let Some(existing) = fakers.get(&instance_id) {
        // Use cumulative stats from previous session
        config_with_cumulative.initial_uploaded = existing.cumulative_uploaded;
        config_with_cumulative.initial_downloaded = existing.cumulative_downloaded;
        log_and_emit!(
            &app,
            instance_id,
            info,
            "Continuing with cumulative stats: uploaded={} bytes, downloaded={} bytes",
            existing.cumulative_uploaded,
            existing.cumulative_downloaded
        );
    }
    drop(fakers); // Release read lock before acquiring write lock

    // Extract cumulative values before moving config
    let cumulative_uploaded = config_with_cumulative.initial_uploaded;
    let cumulative_downloaded = config_with_cumulative.initial_downloaded;

    // Create faker
    let mut faker = RatioFaker::new(torrent, config_with_cumulative).map_err(|e| {
        let error_msg = format!("Failed to create faker: {}", e);
        log_and_emit!(&app, instance_id, error, "{}", error_msg);
        error_msg
    })?;

    // Start the session
    faker.start().await.map_err(|e| {
        let error_msg = format!("Failed to start faker: {}", e);
        log_and_emit!(&app, instance_id, error, "{}", error_msg);
        error_msg
    })?;

    // Store in state with cumulative stats
    let mut fakers = state.fakers.write().await;

    fakers.insert(
        instance_id,
        FakerInstance {
            faker,
            torrent_name,
            cumulative_uploaded,
            cumulative_downloaded,
        },
    );

    log_and_emit!(&app, instance_id, info, "Faker started successfully");
    Ok(())
}

// Tauri command: Stop ratio faking for an instance
#[tauri::command]
async fn stop_faker(instance_id: u32, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    log_and_emit!(&app, instance_id, info, "Stopping faker");

    // Set instance context for logging
    rustatio_core::logger::set_instance_context(Some(instance_id));

    let mut fakers = state.fakers.write().await;

    if let Some(instance) = fakers.get_mut(&instance_id) {
        // Get final stats before stopping to save cumulative totals
        let final_stats = instance.faker.get_stats().await;

        instance.faker.stop().await.map_err(|e| {
            let error_msg = format!("Failed to stop faker: {}", e);
            log_and_emit!(&app, instance_id, error, "{}", error_msg);
            error_msg
        })?;

        // Update cumulative stats in instance (for next session)
        instance.cumulative_uploaded = final_stats.uploaded;
        instance.cumulative_downloaded = final_stats.downloaded;

        log_and_emit!(
            &app,
            instance_id,
            info,
            "Faker stopped successfully - Cumulative: uploaded={} bytes, downloaded={} bytes",
            instance.cumulative_uploaded,
            instance.cumulative_downloaded
        );

        Ok(())
    } else {
        let error_msg = format!("Instance {} not found", instance_id);
        log_and_emit!(&app, warn, "{}", error_msg);
        Err(error_msg)
    }
}

// Tauri command: Update faker stats for an instance
#[tauri::command]
async fn update_faker(instance_id: u32, state: State<'_, AppState>) -> Result<(), String> {
    // Set instance context for logging
    rustatio_core::logger::set_instance_context(Some(instance_id));

    let mut fakers = state.fakers.write().await;

    if let Some(instance) = fakers.get_mut(&instance_id) {
        instance
            .faker
            .update()
            .await
            .map_err(|e| format!("Failed to update faker: {}", e))?;
        Ok(())
    } else {
        Err(format!("Instance {} not found", instance_id))
    }
}

// Tauri command: Update stats only (no tracker update) for an instance
#[tauri::command]
async fn update_stats_only(instance_id: u32, state: State<'_, AppState>) -> Result<FakerStats, String> {
    // Set instance context for logging
    rustatio_core::logger::set_instance_context(Some(instance_id));

    let mut fakers = state.fakers.write().await;

    if let Some(instance) = fakers.get_mut(&instance_id) {
        instance
            .faker
            .update_stats_only()
            .await
            .map_err(|e| format!("Failed to update stats: {}", e))?;
        Ok(instance.faker.get_stats().await)
    } else {
        Err(format!("Instance {} not found", instance_id))
    }
}

// Tauri command: Get current stats for an instance
#[tauri::command]
async fn get_stats(instance_id: u32, state: State<'_, AppState>) -> Result<FakerStats, String> {
    let fakers = state.fakers.read().await;

    if let Some(instance) = fakers.get(&instance_id) {
        Ok(instance.faker.get_stats().await)
    } else {
        Err(format!("Instance {} not found", instance_id))
    }
}

// Tauri command: Scrape tracker for an instance
#[tauri::command]
async fn scrape_tracker(instance_id: u32, state: State<'_, AppState>) -> Result<(i64, i64, i64), String> {
    // Set instance context for logging
    rustatio_core::logger::set_instance_context(Some(instance_id));

    let fakers = state.fakers.read().await;

    if let Some(instance) = fakers.get(&instance_id) {
        let scrape = instance
            .faker
            .scrape()
            .await
            .map_err(|e| format!("Failed to scrape: {}", e))?;
        Ok((scrape.complete, scrape.incomplete, scrape.downloaded))
    } else {
        Err(format!("Instance {} not found", instance_id))
    }
}

// Tauri command: Pause faker for an instance
#[tauri::command]
async fn pause_faker(instance_id: u32, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    log_and_emit!(&app, instance_id, info, "Pausing faker");

    // Set instance context for logging
    rustatio_core::logger::set_instance_context(Some(instance_id));

    let mut fakers = state.fakers.write().await;

    if let Some(instance) = fakers.get_mut(&instance_id) {
        instance
            .faker
            .pause()
            .await
            .map_err(|e| format!("Failed to pause faker: {}", e))?;
        log_and_emit!(&app, instance_id, info, "Faker paused successfully");
        Ok(())
    } else {
        Err(format!("Instance {} not found", instance_id))
    }
}

// Tauri command: Resume faker for an instance
#[tauri::command]
async fn resume_faker(instance_id: u32, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    log_and_emit!(&app, instance_id, info, "Resuming faker");

    // Set instance context for logging
    rustatio_core::logger::set_instance_context(Some(instance_id));

    let mut fakers = state.fakers.write().await;

    if let Some(instance) = fakers.get_mut(&instance_id) {
        instance
            .faker
            .resume()
            .await
            .map_err(|e| format!("Failed to resume faker: {}", e))?;
        log_and_emit!(&app, instance_id, info, "Faker resumed successfully");
        Ok(())
    } else {
        Err(format!("Instance {} not found", instance_id))
    }
}

// Tauri command: Get available client types
#[tauri::command]
async fn get_client_types() -> Vec<String> {
    vec![
        "utorrent".to_string(),
        "qbittorrent".to_string(),
        "transmission".to_string(),
        "deluge".to_string(),
    ]
}

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Rustatio v{} (Multi-Instance)", env!("CARGO_PKG_VERSION"));

    // Load or create default configuration
    let config = AppConfig::load_or_default();

    // Create app state with multi-instance support
    let app_state = AppState {
        fakers: Arc::new(RwLock::new(HashMap::new())),
        next_instance_id: Arc::new(RwLock::new(1)),
        config: Arc::new(RwLock::new(config)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            create_instance,
            delete_instance,
            list_instances,
            load_torrent,
            get_config,
            update_config,
            start_faker,
            stop_faker,
            update_faker,
            update_stats_only,
            get_stats,
            scrape_tracker,
            pause_faker,
            resume_faker,
            get_client_types,
        ])
        .setup(|app| {
            // Initialize the logger with app handle
            rustatio_core::logger::init_logger(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
