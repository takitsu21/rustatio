use rustatio_core::*;
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

// Re-export the set_log_callback function from rustatio_core (WASM only)
#[cfg(target_arch = "wasm32")]
pub use rustatio_core::logger::set_log_callback;

// Instance data with cumulative stats tracking
struct WasmFakerInstance {
    faker: RatioFaker,
    torrent_name: String,
    // Info hash to detect torrent changes
    torrent_info_hash: [u8; 20],
    // Cumulative stats across all sessions for this instance
    cumulative_uploaded: u64,
    cumulative_downloaded: u64,
}

// Global instance storage (using RefCell for single-threaded WASM)
thread_local! {
    #[allow(clippy::missing_const_for_thread_local)]
    static INSTANCES: RefCell<HashMap<u32, WasmFakerInstance>> = RefCell::new(HashMap::new());
    static NEXT_ID: RefCell<u32> = const { RefCell::new(1) };
}

// Helper function to take an instance out of storage
fn take_instance(id: u32) -> Result<WasmFakerInstance, JsValue> {
    INSTANCES.with(|instances| {
        instances
            .borrow_mut()
            .remove(&id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))
    })
}

// Helper function to put an instance back into storage
fn put_instance(id: u32, instance: WasmFakerInstance) {
    INSTANCES.with(|instances| {
        instances.borrow_mut().insert(id, instance);
    });
}

// Helper function to execute an async operation on an instance
// Takes ownership of the instance, passes it to the closure, and expects it back
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

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn create_instance() -> u32 {
    NEXT_ID.with(|next_id| {
        let mut id_ref = next_id.borrow_mut();
        let id = *id_ref;
        *id_ref += 1;
        id
    })
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

    serde_wasm_bindgen::to_value(&torrent).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub async fn start_faker(id: u32, torrent_json: JsValue, config_json: JsValue) -> Result<(), JsValue> {
    // Set instance context for logging
    rustatio_core::logger::set_instance_context(Some(id));

    let torrent: TorrentInfo =
        serde_wasm_bindgen::from_value(torrent_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut config: FakerConfig =
        serde_wasm_bindgen::from_value(config_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Extract torrent info before it's consumed
    let torrent_name = torrent.name.clone();
    let torrent_info_hash = torrent.info_hash;

    // Check if instance already exists - use cumulative stats only if same torrent
    let (cumulative_uploaded, cumulative_downloaded) = INSTANCES.with(|instances| {
        let instances_ref = instances.borrow();
        if let Some(existing) = instances_ref.get(&id) {
            // Only preserve cumulative stats if it's the SAME torrent (same info_hash)
            if existing.torrent_info_hash == torrent_info_hash {
                rustatio_core::log_info!(
                    "Same torrent detected - continuing with cumulative stats: uploaded={} bytes, downloaded={} bytes",
                    existing.cumulative_uploaded,
                    existing.cumulative_downloaded
                );
                (existing.cumulative_uploaded, existing.cumulative_downloaded)
            } else {
                rustatio_core::log_info!(
                    "Different torrent detected - resetting cumulative stats (was: {}, now: {})",
                    existing.torrent_name,
                    torrent_name
                );
                (0u64, 0u64)
            }
        } else {
            (0u64, 0u64)
        }
    });

    // Apply cumulative stats to config
    config.initial_uploaded = cumulative_uploaded;
    config.initial_downloaded = cumulative_downloaded;

    let mut faker = RatioFaker::new(torrent, config).map_err(|e| JsValue::from_str(&e.to_string()))?;

    faker.start().await.map_err(|e| JsValue::from_str(&e.to_string()))?;

    INSTANCES.with(|instances| {
        instances.borrow_mut().insert(
            id,
            WasmFakerInstance {
                faker,
                torrent_name,
                torrent_info_hash,
                cumulative_uploaded,
                cumulative_downloaded,
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
        let result = serde_wasm_bindgen::to_value(&stats).map_err(|e| JsValue::from_str(&e.to_string()));
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
        let result = serde_wasm_bindgen::to_value(&stats).map_err(|e| JsValue::from_str(&e.to_string()));
        (instance, result)
    })
    .await
}

#[wasm_bindgen]
pub async fn get_stats(id: u32) -> Result<JsValue, JsValue> {
    with_instance(id, |instance| async move {
        let stats = instance.faker.get_stats().await;
        let result = serde_wasm_bindgen::to_value(&stats).map_err(|e| JsValue::from_str(&e.to_string()));
        (instance, result)
    })
    .await
}

#[wasm_bindgen]
pub async fn stop_faker(id: u32) -> Result<(), JsValue> {
    rustatio_core::logger::set_instance_context(Some(id));
    with_instance(id, |mut instance| async move {
        // Get final stats before stopping to save cumulative totals
        let final_stats = instance.faker.get_stats().await;

        let result = instance
            .faker
            .stop()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()));

        // Update cumulative stats in instance (for next session)
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
                let result =
                    serde_wasm_bindgen::to_value(&scrape_response).map_err(|e| JsValue::from_str(&e.to_string()));
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
    serde_wasm_bindgen::to_value(&types).unwrap()
}

#[wasm_bindgen]
pub fn get_client_infos() -> JsValue {
    let infos = ClientType::all_infos();
    serde_wasm_bindgen::to_value(&infos).unwrap()
}
