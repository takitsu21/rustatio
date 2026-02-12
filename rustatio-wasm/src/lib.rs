use rustatio_core::*;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

// Re-export the set_log_callback function from rustatio_core (WASM only)
#[cfg(target_arch = "wasm32")]
pub use rustatio_core::logger::set_log_callback;

/// Serialize to JsValue with maps as plain JS objects (not Map instances).
/// serde_wasm_bindgen's default serializes serde_json::Value::Object as JS Map,
/// which breaks property access like `result.imported` in JavaScript.
fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

struct WasmFakerInstance {
    faker: RatioFaker,
    torrent: TorrentInfo,
    config: FakerConfig,
    torrent_info_hash: [u8; 20],
    cumulative_uploaded: u64,
    cumulative_downloaded: u64,
    tags: Vec<String>,
    created_at: u64,
}

thread_local! {
    #[allow(clippy::missing_const_for_thread_local)]
    static INSTANCES: RefCell<HashMap<u32, WasmFakerInstance>> = RefCell::new(HashMap::new());
    static NEXT_ID: RefCell<u32> = const { RefCell::new(1) };
}

fn take_instance(id: u32) -> Result<WasmFakerInstance, JsValue> {
    INSTANCES.with(|instances| {
        instances
            .borrow_mut()
            .remove(&id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))
    })
}

fn put_instance(id: u32, instance: WasmFakerInstance) {
    INSTANCES.with(|instances| {
        instances.borrow_mut().insert(id, instance);
    });
}

async fn with_instance<F, Fut, T>(id: u32, f: F) -> Result<T, JsValue>
where
    F: FnOnce(WasmFakerInstance) -> Fut,
    Fut: std::future::Future<Output = (WasmFakerInstance, Result<T, JsValue>)>,
{
    let instance = take_instance(id)?;
    let (instance, result) = f(instance).await;
    put_instance(id, instance);
    result
}

fn now_millis() -> u64 {
    js_sys::Date::now() as u64
}

fn allocate_id() -> u32 {
    NEXT_ID.with(|next_id| {
        let mut id_ref = next_id.borrow_mut();
        let id = *id_ref;
        *id_ref += 1;
        id
    })
}

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn create_instance() -> u32 {
    allocate_id()
}

#[wasm_bindgen]
pub fn delete_instance(id: u32) -> Result<(), JsValue> {
    INSTANCES.with(|instances| {
        instances.borrow_mut().remove(&id);
        Ok(())
    })
}

#[wasm_bindgen]
pub fn load_torrent(file_bytes: &[u8]) -> Result<JsValue, JsValue> {
    rustatio_core::log_info!("Loading torrent file ({} bytes)", file_bytes.len());

    let torrent = TorrentInfo::from_bytes(file_bytes).map_err(|e| {
        let error_msg = format!("Failed to load torrent: {}", e);
        rustatio_core::log_error!("{}", error_msg);
        JsValue::from_str(&error_msg)
    })?;

    rustatio_core::log_info!("Torrent loaded: {} ({} bytes)", torrent.name, torrent.total_size);

    to_js(&torrent)
}

#[wasm_bindgen]
pub fn load_instance_torrent(id: u32, file_bytes: &[u8]) -> Result<JsValue, JsValue> {
    rustatio_core::logger::set_instance_context(Some(id));
    rustatio_core::log_info!("Loading torrent for instance {} ({} bytes)", id, file_bytes.len());

    let torrent = TorrentInfo::from_bytes(file_bytes).map_err(|e| {
        let error_msg = format!("Failed to load torrent: {}", e);
        rustatio_core::log_error!("{}", error_msg);
        JsValue::from_str(&error_msg)
    })?;

    let torrent_info_hash = torrent.info_hash;
    let config = FakerConfig::default();

    let faker = RatioFaker::new(torrent.clone(), config.clone()).map_err(|e| JsValue::from_str(&e.to_string()))?;

    INSTANCES.with(|instances| {
        instances.borrow_mut().insert(
            id,
            WasmFakerInstance {
                faker,
                torrent: torrent.clone(),
                config,
                torrent_info_hash,
                cumulative_uploaded: 0,
                cumulative_downloaded: 0,
                tags: Vec::new(),
                created_at: now_millis(),
            },
        );
    });

    rustatio_core::log_info!(
        "Torrent loaded for instance {}: {} ({} bytes)",
        id,
        torrent.name,
        torrent.total_size
    );

    to_js(&torrent)
}

#[wasm_bindgen]
pub fn update_instance_config(id: u32, config_json: JsValue) -> Result<(), JsValue> {
    let config: FakerConfig =
        serde_wasm_bindgen::from_value(config_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    INSTANCES.with(|instances| {
        let mut instances_ref = instances.borrow_mut();
        let instance = instances_ref
            .get_mut(&id)
            .ok_or_else(|| JsValue::from_str(&format!("Instance {} not found", id)))?;

        // Recreate the faker so stats reflect the new config
        let faker = RatioFaker::new(instance.torrent.clone(), config.clone())
            .map_err(|e| JsValue::from_str(&format!("Failed to create faker: {}", e)))?;
        instance.config = config;
        instance.faker = faker;
        Ok(())
    })
}

#[wasm_bindgen]
pub async fn start_faker(id: u32, torrent_json: JsValue, config_json: JsValue) -> Result<(), JsValue> {
    rustatio_core::logger::set_instance_context(Some(id));

    let torrent: TorrentInfo =
        serde_wasm_bindgen::from_value(torrent_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut config: FakerConfig =
        serde_wasm_bindgen::from_value(config_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let torrent_info_hash = torrent.info_hash;

    // Preserve cumulative stats if same torrent
    let (cumulative_uploaded, cumulative_downloaded, tags, created_at) = INSTANCES.with(|instances| {
        let instances_ref = instances.borrow();
        if let Some(existing) = instances_ref.get(&id) {
            if existing.torrent_info_hash == torrent_info_hash {
                rustatio_core::log_info!(
                    "Same torrent detected - continuing with cumulative stats: uploaded={} bytes, downloaded={} bytes",
                    existing.cumulative_uploaded,
                    existing.cumulative_downloaded
                );
                (
                    existing.cumulative_uploaded,
                    existing.cumulative_downloaded,
                    existing.tags.clone(),
                    existing.created_at,
                )
            } else {
                rustatio_core::log_info!(
                    "Different torrent detected - resetting cumulative stats (was: {}, now: {})",
                    existing.torrent.name,
                    torrent.name
                );
                (0u64, 0u64, Vec::new(), now_millis())
            }
        } else {
            (0u64, 0u64, Vec::new(), now_millis())
        }
    });

    config.initial_uploaded = cumulative_uploaded;
    config.initial_downloaded = cumulative_downloaded;

    let stored_config = config.clone();
    let mut faker = RatioFaker::new(torrent.clone(), config).map_err(|e| JsValue::from_str(&e.to_string()))?;

    faker.start().await.map_err(|e| JsValue::from_str(&e.to_string()))?;

    INSTANCES.with(|instances| {
        instances.borrow_mut().insert(
            id,
            WasmFakerInstance {
                faker,
                torrent,
                config: stored_config,
                torrent_info_hash,
                cumulative_uploaded,
                cumulative_downloaded,
                tags,
                created_at,
            },
        );
    });

    Ok(())
}

#[wasm_bindgen]
pub async fn update_faker(id: u32) -> Result<JsValue, JsValue> {
    rustatio_core::logger::set_instance_context(Some(id));
    with_instance(id, |mut instance| async move {
        let result = instance.faker.update().await;
        if let Err(e) = result {
            return (instance, Err(JsValue::from_str(&e.to_string())));
        }
        let stats = instance.faker.get_stats().await;
        let result = to_js(&stats);
        (instance, result)
    })
    .await
}

#[wasm_bindgen]
pub async fn update_stats_only(id: u32) -> Result<JsValue, JsValue> {
    rustatio_core::logger::set_instance_context(Some(id));
    with_instance(id, |mut instance| async move {
        let result = instance.faker.update_stats_only().await;
        if let Err(e) = result {
            return (instance, Err(JsValue::from_str(&e.to_string())));
        }
        let stats = instance.faker.get_stats().await;
        let result = to_js(&stats);
        (instance, result)
    })
    .await
}

#[wasm_bindgen]
pub async fn get_stats(id: u32) -> Result<JsValue, JsValue> {
    with_instance(id, |instance| async move {
        let stats = instance.faker.get_stats().await;
        let result = to_js(&stats);
        (instance, result)
    })
    .await
}

#[wasm_bindgen]
pub async fn stop_faker(id: u32) -> Result<(), JsValue> {
    rustatio_core::logger::set_instance_context(Some(id));
    with_instance(id, |mut instance| async move {
        let final_stats = instance.faker.get_stats().await;

        let result = instance
            .faker
            .stop()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()));

        instance.cumulative_uploaded = final_stats.uploaded;
        instance.cumulative_downloaded = final_stats.downloaded;

        rustatio_core::log_info!(
            "Faker stopped - Cumulative: uploaded={} bytes, downloaded={} bytes",
            instance.cumulative_uploaded,
            instance.cumulative_downloaded
        );

        (instance, result)
    })
    .await
}

#[wasm_bindgen]
pub async fn pause_faker(id: u32) -> Result<(), JsValue> {
    rustatio_core::logger::set_instance_context(Some(id));
    with_instance(id, |mut instance| async move {
        let result = instance
            .faker
            .pause()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()));
        (instance, result)
    })
    .await
}

#[wasm_bindgen]
pub async fn resume_faker(id: u32) -> Result<(), JsValue> {
    rustatio_core::logger::set_instance_context(Some(id));
    with_instance(id, |mut instance| async move {
        let result = instance
            .faker
            .resume()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()));
        (instance, result)
    })
    .await
}

#[wasm_bindgen]
pub async fn scrape_tracker(id: u32) -> Result<JsValue, JsValue> {
    rustatio_core::logger::set_instance_context(Some(id));
    with_instance(id, |instance| async move {
        let scrape_result = instance.faker.scrape().await;
        match scrape_result {
            Ok(scrape_response) => {
                let result = to_js(&scrape_response);
                (instance, result)
            }
            Err(e) => (instance, Err(JsValue::from_str(&e.to_string()))),
        }
    })
    .await
}

#[wasm_bindgen]
pub fn get_client_types() -> JsValue {
    let types = ClientType::all_ids();
    to_js(&types).unwrap()
}

#[wasm_bindgen]
pub fn get_instance_torrent(id: u32) -> Result<JsValue, JsValue> {
    INSTANCES.with(|instances| {
        let instances_ref = instances.borrow();
        let instance = instances_ref
            .get(&id)
            .ok_or_else(|| JsValue::from_str(&format!("Instance {} not found", id)))?;
        to_js(&instance.torrent)
    })
}

#[wasm_bindgen]
pub fn get_client_infos() -> JsValue {
    let infos = ClientType::all_infos();
    to_js(&infos).unwrap()
}

// --- Grid Operations ---

#[wasm_bindgen]
pub async fn grid_import(torrent_files: JsValue, config_json: JsValue) -> Result<JsValue, JsValue> {
    let files: Vec<Vec<u8>> =
        serde_wasm_bindgen::from_value(torrent_files).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let settings: GridImportSettings =
        serde_wasm_bindgen::from_value(config_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut imported: Vec<serde_json::Value> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for file_bytes in &files {
        let torrent = match TorrentInfo::from_bytes(file_bytes) {
            Ok(t) => t,
            Err(e) => {
                errors.push(format!("Failed to parse torrent: {}", e));
                continue;
            }
        };

        let id = allocate_id();
        let preset = settings.resolve_for_instance();
        let mut config: FakerConfig = preset.into();
        config.initial_uploaded = 0;
        config.initial_downloaded = 0;

        let name = torrent.name.clone();
        let info_hash_hex = torrent
            .info_hash
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        let torrent_info_hash = torrent.info_hash;
        let total_size = torrent.total_size;

        match RatioFaker::new(torrent.clone(), config.clone()) {
            Ok(mut faker) => {
                if settings.auto_start {
                    if let Err(e) = faker.start().await {
                        errors.push(format!("{}: {}", name, e));
                        continue;
                    }
                }

                INSTANCES.with(|instances| {
                    instances.borrow_mut().insert(
                        id,
                        WasmFakerInstance {
                            faker,
                            torrent,
                            config,
                            torrent_info_hash,
                            cumulative_uploaded: 0,
                            cumulative_downloaded: 0,
                            tags: settings.tags.clone(),
                            created_at: now_millis(),
                        },
                    );
                });

                imported.push(serde_json::json!({
                    "id": id.to_string(),
                    "name": name,
                    "infoHash": info_hash_hex,
                    "totalSize": total_size,
                }));
            }
            Err(e) => {
                errors.push(format!("{}: {}", name, e));
            }
        }
    }

    let result = serde_json::json!({ "imported": imported, "errors": errors });
    to_js(&result)
}

#[wasm_bindgen]
pub async fn grid_start(ids_json: JsValue) -> Result<JsValue, JsValue> {
    let ids: Vec<u32> = serde_wasm_bindgen::from_value(ids_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut succeeded: Vec<String> = Vec::new();
    let mut failed: Vec<serde_json::Value> = Vec::new();

    for id in ids {
        match take_instance(id) {
            Ok(mut instance) => {
                match instance.faker.start().await {
                    Ok(_) => succeeded.push(id.to_string()),
                    Err(e) => failed.push(serde_json::json!({ "id": id.to_string(), "error": e.to_string() })),
                }
                put_instance(id, instance);
            }
            Err(_) => {
                failed.push(serde_json::json!({ "id": id.to_string(), "error": format!("Instance {} not found", id) }));
            }
        }
    }

    let result = serde_json::json!({ "succeeded": succeeded, "failed": failed });
    to_js(&result)
}

#[wasm_bindgen]
pub async fn grid_stop(ids_json: JsValue) -> Result<JsValue, JsValue> {
    let ids: Vec<u32> = serde_wasm_bindgen::from_value(ids_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut succeeded: Vec<String> = Vec::new();
    let mut failed: Vec<serde_json::Value> = Vec::new();

    for id in ids {
        match take_instance(id) {
            Ok(mut instance) => {
                let final_stats = instance.faker.get_stats().await;
                match instance.faker.stop().await {
                    Ok(_) => {
                        instance.cumulative_uploaded = final_stats.uploaded;
                        instance.cumulative_downloaded = final_stats.downloaded;
                        succeeded.push(id.to_string());
                    }
                    Err(e) => failed.push(serde_json::json!({ "id": id.to_string(), "error": e.to_string() })),
                }
                put_instance(id, instance);
            }
            Err(_) => {
                failed.push(serde_json::json!({ "id": id.to_string(), "error": format!("Instance {} not found", id) }));
            }
        }
    }

    let result = serde_json::json!({ "succeeded": succeeded, "failed": failed });
    to_js(&result)
}

#[wasm_bindgen]
pub async fn grid_pause(ids_json: JsValue) -> Result<JsValue, JsValue> {
    let ids: Vec<u32> = serde_wasm_bindgen::from_value(ids_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut succeeded: Vec<String> = Vec::new();
    let mut failed: Vec<serde_json::Value> = Vec::new();

    for id in ids {
        match take_instance(id) {
            Ok(mut instance) => {
                match instance.faker.pause().await {
                    Ok(_) => succeeded.push(id.to_string()),
                    Err(e) => failed.push(serde_json::json!({ "id": id.to_string(), "error": e.to_string() })),
                }
                put_instance(id, instance);
            }
            Err(_) => {
                failed.push(serde_json::json!({ "id": id.to_string(), "error": format!("Instance {} not found", id) }));
            }
        }
    }

    let result = serde_json::json!({ "succeeded": succeeded, "failed": failed });
    to_js(&result)
}

#[wasm_bindgen]
pub async fn grid_resume(ids_json: JsValue) -> Result<JsValue, JsValue> {
    let ids: Vec<u32> = serde_wasm_bindgen::from_value(ids_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut succeeded: Vec<String> = Vec::new();
    let mut failed: Vec<serde_json::Value> = Vec::new();

    for id in ids {
        match take_instance(id) {
            Ok(mut instance) => {
                match instance.faker.resume().await {
                    Ok(_) => succeeded.push(id.to_string()),
                    Err(e) => failed.push(serde_json::json!({ "id": id.to_string(), "error": e.to_string() })),
                }
                put_instance(id, instance);
            }
            Err(_) => {
                failed.push(serde_json::json!({ "id": id.to_string(), "error": format!("Instance {} not found", id) }));
            }
        }
    }

    let result = serde_json::json!({ "succeeded": succeeded, "failed": failed });
    to_js(&result)
}

#[wasm_bindgen]
pub fn grid_delete(ids_json: JsValue) -> Result<JsValue, JsValue> {
    let ids: Vec<u32> = serde_wasm_bindgen::from_value(ids_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut succeeded: Vec<String> = Vec::new();
    let mut failed: Vec<serde_json::Value> = Vec::new();

    INSTANCES.with(|instances| {
        let mut instances_ref = instances.borrow_mut();
        for id in ids {
            if instances_ref.remove(&id).is_some() {
                succeeded.push(id.to_string());
            } else {
                failed.push(serde_json::json!({ "id": id.to_string(), "error": format!("Instance {} not found", id) }));
            }
        }
    });

    let result = serde_json::json!({ "succeeded": succeeded, "failed": failed });
    to_js(&result)
}

#[wasm_bindgen]
pub fn grid_update_config(ids_json: JsValue, config_json: JsValue) -> Result<JsValue, JsValue> {
    let ids: Vec<u32> = serde_wasm_bindgen::from_value(ids_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let preset: PresetSettings =
        serde_wasm_bindgen::from_value(config_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let faker_config: FakerConfig = preset.into();

    let mut succeeded: Vec<String> = Vec::new();
    let mut failed: Vec<serde_json::Value> = Vec::new();

    INSTANCES.with(|instances| {
        let mut instances_ref = instances.borrow_mut();
        for id in ids {
            match instances_ref.get_mut(&id) {
                Some(instance) => {
                    let mut new_config = faker_config.clone();
                    new_config.initial_uploaded = instance.cumulative_uploaded;
                    new_config.initial_downloaded = instance.cumulative_downloaded;

                    match RatioFaker::new(instance.torrent.clone(), new_config) {
                        Ok(faker) => {
                            instance.faker = faker;
                            instance.config = faker_config.clone();
                            succeeded.push(id.to_string());
                        }
                        Err(e) => {
                            failed.push(serde_json::json!({ "id": id.to_string(), "error": e.to_string() }));
                        }
                    }
                }
                None => {
                    failed.push(
                        serde_json::json!({ "id": id.to_string(), "error": format!("Instance {} not found", id) }),
                    );
                }
            }
        }
    });

    let result = serde_json::json!({ "succeeded": succeeded, "failed": failed });
    to_js(&result)
}

#[wasm_bindgen]
pub fn grid_tag(ids_json: JsValue, add_tags: JsValue, remove_tags: JsValue) -> Result<JsValue, JsValue> {
    let ids: Vec<u32> = serde_wasm_bindgen::from_value(ids_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let add: Vec<String> = serde_wasm_bindgen::from_value(add_tags).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let remove: Vec<String> =
        serde_wasm_bindgen::from_value(remove_tags).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut updated = 0u32;

    INSTANCES.with(|instances| {
        let mut instances_ref = instances.borrow_mut();
        for id in ids {
            if let Some(instance) = instances_ref.get_mut(&id) {
                for tag in &remove {
                    instance.tags.retain(|t| t != tag);
                }
                for tag in &add {
                    if !instance.tags.contains(tag) {
                        instance.tags.push(tag.clone());
                    }
                }
                updated += 1;
            }
        }
    });

    let result = serde_json::json!({ "updated": updated });
    to_js(&result)
}

#[wasm_bindgen]
pub fn set_instance_tags(id: u32, tags_json: JsValue) -> Result<(), JsValue> {
    let tags: Vec<String> = serde_wasm_bindgen::from_value(tags_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    INSTANCES.with(|instances| {
        let mut instances_ref = instances.borrow_mut();
        match instances_ref.get_mut(&id) {
            Some(instance) => {
                instance.tags = tags;
                Ok(())
            }
            None => Err(JsValue::from_str(&format!("Instance {} not found", id))),
        }
    })
}

#[wasm_bindgen]
pub async fn list_summaries() -> Result<JsValue, JsValue> {
    let mut summaries: Vec<InstanceSummary> = Vec::new();

    // Collect IDs first to avoid borrow issues with async get_stats
    let ids: Vec<u32> = INSTANCES.with(|instances| instances.borrow().keys().copied().collect());

    for id in ids {
        let instance = take_instance(id)?;
        let stats = instance.faker.get_stats().await;
        let info_hash_hex = instance
            .torrent_info_hash
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        summaries.push(InstanceSummary {
            id: id.to_string(),
            name: instance.torrent.name.clone(),
            info_hash: info_hash_hex,
            state: match stats.state {
                FakerState::Paused => "paused".to_string(),
                _ if stats.is_idling => "idle".to_string(),
                _ => format!("{:?}", stats.state).to_lowercase(),
            },
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
            source: "manual".to_string(),
            created_at: instance.created_at,
        });

        put_instance(id, instance);
    }

    to_js(&summaries)
}
