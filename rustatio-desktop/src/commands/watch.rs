use rustatio_core::{FakerConfig, PresetSettings};
use rustatio_watch::{WatchStatus, WatchedFile};
use std::sync::Arc;
use tauri::State;

use crate::persistence::WatchSettings;
use crate::state::AppState;

#[tauri::command]
pub async fn get_watch_status(state: State<'_, AppState>) -> Result<WatchStatus, String> {
    let watch_guard = state.watch.read().await;
    if let Some(watch) = watch_guard.as_ref() {
        let status = watch.get_status().await;
        return Ok(status);
    }

    let settings = state.watch_settings.read().await.clone().unwrap_or_default();
    let watch_dir = crate::watch::default_watch_dir(Some(&settings));
    Ok(WatchStatus {
        enabled: false,
        watch_dir: watch_dir.to_string_lossy().to_string(),
        auto_start: settings.auto_start,
        file_count: 0,
        loaded_count: 0,
    })
}

#[tauri::command]
pub async fn list_watch_files(state: State<'_, AppState>) -> Result<Vec<WatchedFile>, String> {
    let watch_guard = state.watch.read().await;
    if let Some(watch) = watch_guard.as_ref() {
        let files = watch.list_files().await;
        return Ok(files);
    }

    Ok(Vec::new())
}

#[tauri::command]
pub async fn delete_watch_file(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let watch_guard = state.watch.read().await;
    let watch = watch_guard.as_ref().ok_or_else(|| "Watch service not initialized".to_string())?;
    let result = watch.delete_file(&path).await;
    result
}

#[tauri::command]
pub async fn reload_watch_file(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let watch_guard = state.watch.read().await;
    let watch = watch_guard.as_ref().ok_or_else(|| "Watch service not initialized".to_string())?;
    let result = watch.reload_file(&path).await;
    result
}

#[tauri::command]
pub async fn reload_all_watch_files(state: State<'_, AppState>) -> Result<u32, String> {
    let watch_guard = state.watch.read().await;
    let watch = watch_guard.as_ref().ok_or_else(|| "Watch service not initialized".to_string())?;
    let count = watch.reload_all().await?;
    Ok(count)
}

#[tauri::command]
pub async fn get_watch_config(state: State<'_, AppState>) -> Result<WatchSettings, String> {
    let settings = state.watch_settings.read().await;
    let mut config = settings.clone().unwrap_or_default();
    if config.watch_dir.as_ref().is_none_or(|v| v.trim().is_empty()) {
        let path = crate::watch::default_watch_dir(Some(&config));
        config.watch_dir = Some(path.to_string_lossy().to_string());
    }
    Ok(config)
}

#[tauri::command]
pub async fn set_watch_config(
    config: WatchSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let prior = state.watch_settings.read().await.clone().unwrap_or_default();
    let prior_watch_dir = prior.watch_dir.clone();

    let merged_watch_dir = config.watch_dir.map_or(prior_watch_dir, |path| {
        let trimmed = path.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    let merged = WatchSettings {
        max_depth: config.max_depth,
        auto_start: config.auto_start,
        watch_dir: merged_watch_dir,
    };

    let defaults = Arc::clone(&state.default_config);

    {
        let mut settings = state.watch_settings.write().await;
        *settings = Some(merged.clone());
    }

    let norm_path = |path: &Option<String>| {
        path.as_ref().and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    };

    // Watch dir unchanged: apply lightweight runtime updates without restart.
    if norm_path(&prior.watch_dir) == norm_path(&merged.watch_dir) {
        let mut watch_guard = state.watch.write().await;
        if let Some(watch) = watch_guard.as_mut() {
            if prior.max_depth != merged.max_depth {
                watch.set_max_depth(merged.max_depth);
            }
            if prior.auto_start != merged.auto_start {
                watch.set_auto_start(merged.auto_start);
            }
        }

        return state.save_state().await;
    }

    let mut old_watch = {
        let mut watch_guard = state.watch.write().await;
        watch_guard.take()
    };

    if let Some(current_watch) = old_watch.as_mut() {
        current_watch.stop().await;
    }

    let state_clone = state.inner().clone();
    let mut next_watch = crate::watch::build_watch_service(state_clone, defaults, Some(merged));
    if let Err(e) = next_watch.start().await {
        if let Some(mut previous_watch) = old_watch {
            if let Err(restore_err) = previous_watch.start().await {
                log::error!("Failed to restore previous watch service: {restore_err}");
            }

            let mut watch_guard = state.watch.write().await;
            *watch_guard = Some(previous_watch);
        }

        return Err(format!("Failed to start watch service with new settings: {e}"));
    }

    let mut watch_guard = state.watch.write().await;
    *watch_guard = Some(next_watch);

    state.save_state().await
}

#[tauri::command]
pub async fn get_default_config(state: State<'_, AppState>) -> Result<Option<FakerConfig>, String> {
    let config = state.default_config.read().await;
    let result = config.clone();
    Ok(result)
}

#[tauri::command]
pub async fn set_default_config(
    config: PresetSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let updated: FakerConfig = config.into();
    {
        let mut defaults = state.default_config.write().await;
        *defaults = Some(updated);
    }
    state.save_state().await
}

#[tauri::command]
pub async fn clear_default_config(state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut defaults = state.default_config.write().await;
        *defaults = None;
    }
    state.save_state().await
}
