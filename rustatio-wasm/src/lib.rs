use rustatio_core::*;
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

// Re-export the set_log_callback function from rustatio_core (WASM only)
#[cfg(target_arch = "wasm32")]
pub use rustatio_core::logger::set_log_callback;

// Global instance storage (using RefCell for single-threaded WASM)
thread_local! {
    #[allow(clippy::missing_const_for_thread_local)]
    static INSTANCES: RefCell<HashMap<u32, RatioFaker>> = RefCell::new(HashMap::new());
    static NEXT_ID: RefCell<u32> = const { RefCell::new(1) };
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
    let torrent: TorrentInfo =
        serde_wasm_bindgen::from_value(torrent_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let config: FakerConfig =
        serde_wasm_bindgen::from_value(config_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut faker = RatioFaker::new(torrent, config).map_err(|e| JsValue::from_str(&e.to_string()))?;

    faker.start().await.map_err(|e| JsValue::from_str(&e.to_string()))?;

    INSTANCES.with(|instances| {
        instances.borrow_mut().insert(id, faker);
    });

    Ok(())
}

#[wasm_bindgen]
pub async fn update_faker(id: u32) -> Result<JsValue, JsValue> {
    // Take the faker out temporarily
    let mut faker = INSTANCES.with(|instances| {
        instances
            .borrow_mut()
            .remove(&id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))
    })?;

    // Perform async operation
    faker.update().await.map_err(|e| JsValue::from_str(&e.to_string()))?;

    let stats = faker.get_stats().await;
    let result = serde_wasm_bindgen::to_value(&stats).map_err(|e| JsValue::from_str(&e.to_string()));

    // Put it back
    INSTANCES.with(|instances| {
        instances.borrow_mut().insert(id, faker);
    });

    result
}

#[wasm_bindgen]
pub async fn update_stats_only(id: u32) -> Result<JsValue, JsValue> {
    // Take the faker out temporarily
    let mut faker = INSTANCES.with(|instances| {
        instances
            .borrow_mut()
            .remove(&id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))
    })?;

    // Perform async operation
    faker
        .update_stats_only()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let stats = faker.get_stats().await;
    let result = serde_wasm_bindgen::to_value(&stats).map_err(|e| JsValue::from_str(&e.to_string()));

    // Put it back
    INSTANCES.with(|instances| {
        instances.borrow_mut().insert(id, faker);
    });

    result
}

#[wasm_bindgen]
pub async fn get_stats(id: u32) -> Result<JsValue, JsValue> {
    // Take the faker out temporarily
    let faker = INSTANCES.with(|instances| {
        instances
            .borrow_mut()
            .remove(&id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))
    })?;

    let stats = faker.get_stats().await;
    let result = serde_wasm_bindgen::to_value(&stats).map_err(|e| JsValue::from_str(&e.to_string()));

    // Put it back
    INSTANCES.with(|instances| {
        instances.borrow_mut().insert(id, faker);
    });

    result
}

#[wasm_bindgen]
pub async fn stop_faker(id: u32) -> Result<(), JsValue> {
    // Take the faker out temporarily
    let mut faker = INSTANCES.with(|instances| {
        instances
            .borrow_mut()
            .remove(&id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))
    })?;

    // Perform async operation
    let result = faker.stop().await.map_err(|e| JsValue::from_str(&e.to_string()));

    // Put it back
    INSTANCES.with(|instances| {
        instances.borrow_mut().insert(id, faker);
    });

    result
}

#[wasm_bindgen]
pub async fn pause_faker(id: u32) -> Result<(), JsValue> {
    // Take the faker out temporarily
    let mut faker = INSTANCES.with(|instances| {
        instances
            .borrow_mut()
            .remove(&id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))
    })?;

    // Perform async operation
    let result = faker.pause().await.map_err(|e| JsValue::from_str(&e.to_string()));

    // Put it back
    INSTANCES.with(|instances| {
        instances.borrow_mut().insert(id, faker);
    });

    result
}

#[wasm_bindgen]
pub async fn resume_faker(id: u32) -> Result<(), JsValue> {
    // Take the faker out temporarily
    let mut faker = INSTANCES.with(|instances| {
        instances
            .borrow_mut()
            .remove(&id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))
    })?;

    // Perform async operation
    let result = faker.resume().await.map_err(|e| JsValue::from_str(&e.to_string()));

    // Put it back
    INSTANCES.with(|instances| {
        instances.borrow_mut().insert(id, faker);
    });

    result
}

#[wasm_bindgen]
pub async fn scrape_tracker(id: u32) -> Result<JsValue, JsValue> {
    // Take the faker out temporarily
    let faker = INSTANCES.with(|instances| {
        instances
            .borrow_mut()
            .remove(&id)
            .ok_or_else(|| JsValue::from_str("Instance not found"))
    })?;

    let scrape_response = faker.scrape().await.map_err(|e| JsValue::from_str(&e.to_string()))?;

    let result = serde_wasm_bindgen::to_value(&scrape_response).map_err(|e| JsValue::from_str(&e.to_string()));

    // Put it back
    INSTANCES.with(|instances| {
        instances.borrow_mut().insert(id, faker);
    });

    result
}

#[wasm_bindgen]
pub fn get_client_types() -> JsValue {
    let types = vec!["utorrent", "qbittorrent", "transmission", "deluge"];
    serde_wasm_bindgen::to_value(&types).unwrap()
}
