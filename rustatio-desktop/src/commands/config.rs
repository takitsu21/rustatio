use rustatio_core::validation;
use rustatio_core::{AppConfig, ClientInfo, ClientType};
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let config = state.config.read().await;
    Ok(config.clone())
}

#[tauri::command]
pub async fn update_config(config: AppConfig, state: State<'_, AppState>) -> Result<(), String> {
    validation::validate_rate(config.faker.default_upload_rate, "upload_rate").map_err(|e| format!("{}", e))?;
    validation::validate_rate(config.faker.default_download_rate, "download_rate").map_err(|e| format!("{}", e))?;
    validation::validate_update_interval(config.faker.update_interval).map_err(|e| format!("{}", e))?;
    validation::validate_port(config.client.default_port).map_err(|e| format!("{}", e))?;

    let mut app_config = state.config.write().await;
    *app_config = config.clone();

    let path = AppConfig::default_path();
    config
        .save(&path)
        .map_err(|e| format!("Failed to save config: {}", e))?;

    log::info!("Configuration updated and saved");
    Ok(())
}

#[tauri::command]
pub async fn get_client_types() -> Vec<String> {
    ClientType::all_ids()
}

#[tauri::command]
pub async fn get_client_infos() -> Vec<ClientInfo> {
    ClientType::all_infos()
}

#[tauri::command]
pub async fn write_file(path: String, contents: String) -> Result<(), String> {
    std::fs::write(&path, contents).map_err(|e| format!("Failed to write file: {}", e))?;
    log::info!("File written to: {}", path);
    Ok(())
}
