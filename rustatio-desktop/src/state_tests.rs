#[cfg(test)]
mod tests {
    use crate::persistence;
    use crate::state::{AppState, FakerInstance, InstanceInfo};
    use rustatio_core::{
        AppConfig, FakerConfig, PeerListenerStatus, RatioFaker, RatioFakerHandle, TorrentInfo,
    };
    use rustatio_watch::InstanceSource;
    use std::collections::HashMap;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, OnceLock};
    use tokio::sync::{Mutex, RwLock};

    fn state_path_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn torrent() -> TorrentInfo {
        TorrentInfo {
            info_hash: [3u8; 20],
            announce: "https://tracker.test/announce".to_string(),
            announce_list: None,
            name: "saved-torrent".to_string(),
            total_size: 1024,
            piece_length: 256,
            num_pieces: 4,
            creation_date: None,
            comment: None,
            created_by: None,
            is_single_file: true,
            file_count: 1,
            files: Vec::new(),
        }
    }

    fn app_state() -> AppState {
        AppState {
            fakers: Arc::new(RwLock::new(HashMap::new())),
            next_instance_id: Arc::new(RwLock::new(1)),
            config: Arc::new(RwLock::new(AppConfig::default())),
            http_client: rustatio_core::reqwest::Client::new(),
            watch: Arc::new(RwLock::new(None)),
            default_config: Arc::new(RwLock::new(None)),
            watch_settings: Arc::new(RwLock::new(None)),
            should_exit: Arc::new(AtomicBool::new(false)),
            close_prompt_open: Arc::new(AtomicBool::new(false)),
            peer_listener: Arc::new(RwLock::new(None)),
            peer_listener_status: Arc::new(RwLock::new(PeerListenerStatus::default())),
        }
    }

    async fn insert_instance(state: &AppState, id: u32, config: FakerConfig) {
        let info = torrent();
        let torrent = Arc::new(info.clone());
        let summary = Arc::new(info.summary());
        let faker =
            RatioFaker::new(Arc::clone(&torrent), config.clone(), Some(state.http_client.clone()));
        assert!(faker.is_ok());
        let faker = faker.unwrap_or_else(|_| unreachable!());

        state.fakers.write().await.insert(
            id,
            FakerInstance {
                faker: Arc::new(RatioFakerHandle::new(faker)),
                torrent,
                summary,
                config,
                cumulative_uploaded: 0,
                cumulative_downloaded: 0,
                tags: vec!["alpha".to_string()],
                created_at: 7,
                source: InstanceSource::WatchFolder,
            },
        );
    }

    #[tokio::test]
    async fn build_persisted_state_keeps_instance_config_as_source_of_truth() {
        let state = app_state();
        let config = FakerConfig {
            upload_rate: 321.0,
            port: 51413,
            stop_at_ratio: Some(3.5),
            ..FakerConfig::default()
        };

        insert_instance(&state, 5, config.clone()).await;

        let persisted = state.build_persisted_state().await;
        assert_eq!(persisted.instances.len(), 1);

        let saved = persisted.instances.get(&5);
        assert!(saved.is_some());
        let saved = saved.unwrap_or_else(|| unreachable!());
        assert_eq!(saved.config.upload_rate, config.upload_rate);
        assert_eq!(saved.config.port, config.port);
        assert_eq!(saved.config.stop_at_ratio, config.stop_at_ratio);
        assert!(saved.from_watch_folder);
        assert_eq!(saved.tags, vec!["alpha".to_string()]);
    }

    #[tokio::test]
    async fn build_persisted_state_blocking_matches_async_snapshot() {
        let state = app_state();
        let config = FakerConfig {
            upload_rate: 321.0,
            port: 51413,
            stop_at_ratio: Some(3.5),
            ..FakerConfig::default()
        };

        insert_instance(&state, 5, config).await;

        let async_state = state.build_persisted_state().await;
        let blocking_state = tokio::task::spawn_blocking({
            let state = state.clone();
            move || state.build_persisted_state_blocking()
        })
        .await;
        assert!(blocking_state.is_ok());
        let blocking_state = blocking_state.unwrap_or_else(|_| unreachable!());

        assert_eq!(blocking_state.next_instance_id, async_state.next_instance_id);
        assert_eq!(blocking_state.version, async_state.version);

        let async_default = serde_json::to_value(async_state.default_config);
        assert!(async_default.is_ok());
        let async_default = async_default.unwrap_or_else(|_| unreachable!());

        let blocking_default = serde_json::to_value(blocking_state.default_config);
        assert!(blocking_default.is_ok());
        let blocking_default = blocking_default.unwrap_or_else(|_| unreachable!());
        assert_eq!(blocking_default, async_default);

        let async_watch = serde_json::to_value(async_state.watch_settings);
        assert!(async_watch.is_ok());
        let async_watch = async_watch.unwrap_or_else(|_| unreachable!());

        let blocking_watch = serde_json::to_value(blocking_state.watch_settings);
        assert!(blocking_watch.is_ok());
        let blocking_watch = blocking_watch.unwrap_or_else(|_| unreachable!());
        assert_eq!(blocking_watch, async_watch);

        let async_instance = async_state.instances.get(&5);
        assert!(async_instance.is_some());
        let async_instance = async_instance.unwrap_or_else(|| unreachable!());

        let blocking_instance = blocking_state.instances.get(&5);
        assert!(blocking_instance.is_some());
        let blocking_instance = blocking_instance.unwrap_or_else(|| unreachable!());

        assert_eq!(blocking_instance.id, async_instance.id);
        assert_eq!(blocking_instance.cumulative_uploaded, async_instance.cumulative_uploaded);
        assert_eq!(blocking_instance.cumulative_downloaded, async_instance.cumulative_downloaded);
        assert_eq!(blocking_instance.created_at, async_instance.created_at);
        assert_eq!(blocking_instance.tags, async_instance.tags);
        assert_eq!(blocking_instance.from_watch_folder, async_instance.from_watch_folder);
        assert!(blocking_instance.updated_at >= async_instance.updated_at);
        assert!(blocking_instance.updated_at <= async_instance.updated_at.saturating_add(1));

        let async_torrent = serde_json::to_value(&async_instance.torrent);
        assert!(async_torrent.is_ok());
        let async_torrent = async_torrent.unwrap_or_else(|_| unreachable!());

        let blocking_torrent = serde_json::to_value(&blocking_instance.torrent);
        assert!(blocking_torrent.is_ok());
        let blocking_torrent = blocking_torrent.unwrap_or_else(|_| unreachable!());
        assert_eq!(blocking_torrent, async_torrent);

        let async_config = serde_json::to_value(&async_instance.config);
        assert!(async_config.is_ok());
        let async_config = async_config.unwrap_or_else(|_| unreachable!());

        let blocking_config = serde_json::to_value(&blocking_instance.config);
        assert!(blocking_config.is_ok());
        let blocking_config = blocking_config.unwrap_or_else(|_| unreachable!());
        assert_eq!(blocking_config, async_config);

        let async_state_value = serde_json::to_value(async_instance.state);
        assert!(async_state_value.is_ok());
        let async_state_value = async_state_value.unwrap_or_else(|_| unreachable!());

        let blocking_state_value = serde_json::to_value(blocking_instance.state);
        assert!(blocking_state_value.is_ok());
        let blocking_state_value = blocking_state_value.unwrap_or_else(|_| unreachable!());
        assert_eq!(blocking_state_value, async_state_value);
    }

    #[test]
    fn instance_info_serializes_server_style_shape() {
        let info = InstanceInfo {
            id: "5".to_string(),
            torrent: torrent().summary(),
            config: FakerConfig::default(),
            stats: RatioFaker::stats_from_config(&FakerConfig::default()),
            created_at: 7,
            source: "watch_folder".to_string(),
            tags: vec!["alpha".to_string()],
        };

        let json = serde_json::to_value(info);
        assert!(json.is_ok());
        let json = json.unwrap_or_else(|_| unreachable!());

        assert_eq!(json.get("id").and_then(serde_json::Value::as_str), Some("5"));
        assert!(json.get("torrent").is_some());
        assert!(json.get("config").is_some());
        assert!(json.get("stats").is_some());
        assert_eq!(json.get("source").and_then(serde_json::Value::as_str), Some("watch_folder"));
    }

    #[tokio::test]
    async fn apply_instance_config_saves_updated_state_immediately() {
        let _guard = state_path_test_lock().lock().await;
        let temp = tempfile::tempdir();
        assert!(temp.is_ok());
        let temp = temp.unwrap_or_else(|_| unreachable!());
        let path = temp.path().join("desktop-state.json");
        persistence::set_test_state_file_path(Some(path));

        let state = app_state();
        insert_instance(&state, 5, FakerConfig::default()).await;

        let updated = FakerConfig {
            upload_rate: 444.0,
            download_rate: 222.0,
            port: 51413,
            stop_at_ratio: Some(4.0),
            ..FakerConfig::default()
        };

        let result = state.apply_instance_config(5, updated.clone()).await;
        assert!(result.is_ok());

        let persisted = persistence::load_state();
        let saved = persisted.instances.get(&5);
        assert!(saved.is_some());
        let saved = saved.unwrap_or_else(|| unreachable!());
        assert_eq!(saved.config.upload_rate, updated.upload_rate);
        assert_eq!(saved.config.port, updated.port);
        assert_eq!(saved.config.stop_at_ratio, updated.stop_at_ratio);

        persistence::set_test_state_file_path(None);
    }

    #[tokio::test]
    async fn apply_instance_config_rolls_back_when_save_fails() {
        let _guard = state_path_test_lock().lock().await;
        let temp = tempfile::tempdir();
        assert!(temp.is_ok());
        let temp = temp.unwrap_or_else(|_| unreachable!());
        let blocker = temp.path().join("blocked-parent");
        let blocker_write = std::fs::write(&blocker, b"not a directory");
        assert!(blocker_write.is_ok());
        let path = blocker.join("desktop-state.json");
        persistence::set_test_state_file_path(Some(path));

        let state = app_state();
        let original = FakerConfig { upload_rate: 111.0, port: 40000, ..FakerConfig::default() };
        insert_instance(&state, 5, original.clone()).await;

        let updated = FakerConfig { upload_rate: 444.0, port: 51413, ..FakerConfig::default() };

        let result = state.apply_instance_config(5, updated).await;
        assert!(result.is_err());

        let fakers = state.fakers.read().await;
        let instance = fakers.get(&5);
        assert!(instance.is_some());
        let instance = instance.unwrap_or_else(|| unreachable!());
        assert_eq!(instance.config.upload_rate, original.upload_rate);
        assert_eq!(instance.config.port, original.port);
        assert_eq!(instance.faker.effective_port().await, original.port);

        persistence::set_test_state_file_path(None);
    }
}
