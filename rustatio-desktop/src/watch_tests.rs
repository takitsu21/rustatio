#[cfg(test)]
mod tests {
    use crate::watch::default_watch_dir;
    use std::env;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn default_watch_dir_uses_env() -> Result<(), Box<dyn std::error::Error>> {
        let guard = env_lock().lock();
        if guard.is_err() {
            return Err("failed to acquire env mutex".into());
        }

        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().to_string_lossy().to_string();

        env::set_var("WATCH_DIR", &path);
        let resolved = default_watch_dir(None);
        assert_eq!(resolved, temp_dir.path());
        env::remove_var("WATCH_DIR");
        Ok(())
    }

    #[test]
    fn default_watch_dir_falls_back() {
        let guard = env_lock().lock();
        assert!(guard.is_ok(), "failed to acquire env mutex");

        env::remove_var("WATCH_DIR");
        let resolved = default_watch_dir(None);
        assert_eq!(resolved.to_string_lossy(), "torrents");
    }

    #[test]
    fn default_watch_dir_uses_settings_first() {
        let guard = env_lock().lock();
        assert!(guard.is_ok(), "failed to acquire env mutex");

        env::remove_var("WATCH_DIR");
        let settings = crate::persistence::WatchSettings {
            max_depth: 2,
            auto_start: false,
            watch_dir: Some("/tmp/rustatio-watch".to_string()),
        };
        let resolved = default_watch_dir(Some(&settings));
        assert_eq!(resolved.to_string_lossy(), "/tmp/rustatio-watch");
    }

    #[test]
    fn watch_settings_default_uses_env_auto_start() {
        let guard = env_lock().lock();
        assert!(guard.is_ok(), "failed to acquire env mutex");

        env::set_var("WATCH_AUTO_START", "true");
        let settings = crate::persistence::WatchSettings::default();
        assert!(settings.auto_start);

        env::set_var("WATCH_AUTO_START", "0");
        let settings = crate::persistence::WatchSettings::default();
        assert!(!settings.auto_start);

        env::remove_var("WATCH_AUTO_START");
    }

    #[test]
    fn persisted_instance_from_watch_folder_defaults_false() {
        let instance = crate::persistence::PersistedInstance {
            id: 42,
            torrent: rustatio_core::TorrentSummary::default(),
            config: rustatio_core::FakerConfig::default(),
            cumulative_uploaded: 0,
            cumulative_downloaded: 0,
            state: rustatio_core::FakerState::Stopped,
            created_at: 1,
            updated_at: 1,
            tags: vec![],
            from_watch_folder: false,
        };

        let json = serde_json::to_string(&instance).unwrap_or_else(|_| unreachable!());
        let parsed = serde_json::from_str::<crate::persistence::PersistedInstance>(&json);
        assert!(parsed.is_ok(), "persisted instance should deserialize");
        let loaded = parsed.unwrap_or_else(|_| unreachable!());
        assert!(!loaded.from_watch_folder);
    }
}
