use rustatio_core::validation;
use rustatio_core::{FakerConfig, FakerStats, RatioFaker, TorrentInfo};
use tauri::{AppHandle, State};

use crate::logging::log_and_emit;
use crate::state::{AppState, FakerInstance};

#[tauri::command]
pub async fn start_faker(
    instance_id: u32,
    torrent: TorrentInfo,
    config: FakerConfig,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
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

    let torrent_info_hash = torrent.info_hash;

    rustatio_core::logger::set_instance_context(Some(instance_id));

    // Check if instance already exists (restarting) - preserve cumulative stats
    let mut config_with_cumulative = config.clone();
    let fakers = state.fakers.read().await;
    if let Some(existing) = fakers.get(&instance_id) {
        if existing.torrent.info_hash == torrent_info_hash {
            config_with_cumulative.initial_uploaded = existing.cumulative_uploaded;
            config_with_cumulative.initial_downloaded = existing.cumulative_downloaded;
            let existing_stats = existing.faker.get_stats().await;
            config_with_cumulative.completion_percent = existing_stats.torrent_completion;
            log_and_emit!(
                &app,
                instance_id,
                info,
                "Same torrent detected - continuing with cumulative stats: uploaded={} bytes, downloaded={} bytes, completion={:.1}%",
                existing.cumulative_uploaded,
                existing.cumulative_downloaded,
                existing_stats.torrent_completion
            );
        } else {
            log_and_emit!(
                &app,
                instance_id,
                info,
                "Different torrent detected - resetting cumulative stats (was: {}, now: {})",
                existing.torrent.name,
                torrent.name
            );
        }
    }
    drop(fakers);

    let cumulative_uploaded = config_with_cumulative.initial_uploaded;
    let cumulative_downloaded = config_with_cumulative.initial_downloaded;

    let mut faker = RatioFaker::new(torrent.clone(), config_with_cumulative).map_err(|e| {
        let error_msg = format!("Failed to create faker: {}", e);
        log_and_emit!(&app, instance_id, error, "{}", error_msg);
        error_msg
    })?;

    faker.start().await.map_err(|e| {
        let error_msg = format!("Failed to start faker: {}", e);
        log_and_emit!(&app, instance_id, error, "{}", error_msg);
        error_msg
    })?;

    let mut fakers = state.fakers.write().await;

    let existing = fakers.get(&instance_id);
    let existing_tags = existing.map(|i| i.tags.clone()).unwrap_or_default();
    let created_at = existing.map(|i| i.created_at).unwrap_or_else(crate::state::now_secs);

    fakers.insert(
        instance_id,
        FakerInstance {
            faker,
            torrent,
            config,
            cumulative_uploaded,
            cumulative_downloaded,
            tags: existing_tags,
            created_at,
        },
    );

    log_and_emit!(&app, instance_id, info, "Faker started successfully");
    drop(fakers);
    state.save_state().await;
    Ok(())
}

#[tauri::command]
pub async fn stop_faker(instance_id: u32, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    log_and_emit!(&app, instance_id, info, "Stopping faker");
    rustatio_core::logger::set_instance_context(Some(instance_id));

    let mut fakers = state.fakers.write().await;

    if let Some(instance) = fakers.get_mut(&instance_id) {
        let final_stats = instance.faker.get_stats().await;

        instance.faker.stop().await.map_err(|e| {
            let error_msg = format!("Failed to stop faker: {}", e);
            log_and_emit!(&app, instance_id, error, "{}", error_msg);
            error_msg
        })?;

        instance.cumulative_uploaded = final_stats.uploaded;
        instance.cumulative_downloaded = final_stats.downloaded;
        instance.config.completion_percent = final_stats.torrent_completion;

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
    }?;

    drop(fakers);
    state.save_state().await;
    Ok(())
}

#[tauri::command]
pub async fn update_faker(instance_id: u32, state: State<'_, AppState>) -> Result<(), String> {
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

#[tauri::command]
pub async fn update_stats_only(instance_id: u32, state: State<'_, AppState>) -> Result<FakerStats, String> {
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

#[tauri::command]
pub async fn get_stats(instance_id: u32, state: State<'_, AppState>) -> Result<FakerStats, String> {
    let fakers = state.fakers.read().await;

    if let Some(instance) = fakers.get(&instance_id) {
        Ok(instance.faker.get_stats().await)
    } else {
        Err(format!("Instance {} not found", instance_id))
    }
}

#[tauri::command]
pub async fn scrape_tracker(instance_id: u32, state: State<'_, AppState>) -> Result<(i64, i64, i64), String> {
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

#[tauri::command]
pub async fn pause_faker(instance_id: u32, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    log_and_emit!(&app, instance_id, info, "Pausing faker");
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
    }?;

    drop(fakers);
    state.save_state().await;
    Ok(())
}

#[tauri::command]
pub async fn resume_faker(instance_id: u32, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    log_and_emit!(&app, instance_id, info, "Resuming faker");
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
    }?;

    drop(fakers);
    state.save_state().await;
    Ok(())
}
