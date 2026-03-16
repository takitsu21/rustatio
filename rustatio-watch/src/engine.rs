use crate::paths::relative_watch_path;
use crate::scan::{is_torrent_file, scan_torrent_paths};
use crate::types::{EngineConfig, WatchStatus, WatchedFile, WatchedFileStatus};
use async_trait::async_trait;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rustatio_core::TorrentSummary;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceSource {
    Manual,
    WatchFolder,
}

#[derive(Clone)]
pub struct InstanceState {
    pub id: String,
    pub info_hash: [u8; 20],
    pub source: InstanceSource,
    pub state: String,
    pub name: String,
}

pub struct NewInstance {
    pub id: String,
    pub info: rustatio_core::TorrentInfo,
    pub config: rustatio_core::FakerConfig,
    pub auto_start: bool,
}

#[async_trait]
pub trait WatchEngine: Send + Sync {
    async fn list_instances(&self) -> Vec<InstanceState>;
    async fn create_instance(&self, instance: NewInstance) -> Result<(), String>;
    async fn start_instance(&self, id: &str) -> Result<(), String>;
    async fn delete_instance_by_info_hash(&self, info_hash: &[u8; 20]) -> Result<(), String>;
    async fn find_instance_by_info_hash(&self, info_hash: &[u8; 20]) -> Option<String>;
    async fn update_instance_source_by_info_hash(
        &self,
        info_hash: &[u8; 20],
        source: InstanceSource,
    ) -> Result<(), String>;
    async fn default_config(&self) -> Option<rustatio_core::FakerConfig>;
    fn next_instance_id(&self) -> String;
}

pub struct WatchService<E: WatchEngine> {
    config: EngineConfig,
    engine: Arc<E>,
    loaded_hashes: Arc<RwLock<HashSet<[u8; 20]>>>,
    path_to_hash: Arc<RwLock<HashMap<PathBuf, [u8; 20]>>>,
    max_depth: Arc<AtomicU32>,
    auto_start: Arc<AtomicBool>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

fn resolve_watch_file(watch_dir: &Path, filename: &str) -> Result<(PathBuf, PathBuf), String> {
    let path = watch_dir.join(filename);

    let canonical_watch =
        watch_dir.canonicalize().map_err(|e| format!("Failed to canonicalize watch dir: {e}"))?;
    let canonical_file = path.canonicalize().map_err(|e| format!("File not found: {e}"))?;

    if !canonical_file.starts_with(&canonical_watch) {
        return Err("Invalid file path".to_string());
    }

    let relative = relative_watch_path(&canonical_watch, &canonical_file)?;
    Ok((canonical_file, relative))
}

struct HashDetachCtx<'a, E: WatchEngine> {
    engine: &'a Arc<E>,
    loaded_hashes: &'a Arc<RwLock<HashSet<[u8; 20]>>>,
    path_to_hash: &'a Arc<RwLock<HashMap<PathBuf, [u8; 20]>>>,
}

impl<E: WatchEngine> HashDetachCtx<'_, E> {
    async fn detach(
        &self,
        relative: Option<&PathBuf>,
        info_hash: &[u8; 20],
        set_manual_source: bool,
    ) {
        if let Some(path) = relative {
            self.path_to_hash.write().await.remove(path);
        }
        self.loaded_hashes.write().await.remove(info_hash);

        if set_manual_source {
            if let Err(e) = self
                .engine
                .update_instance_source_by_info_hash(info_hash, InstanceSource::Manual)
                .await
            {
                tracing::warn!("Failed to update instance source: {}", e);
            }
        }

        if let Err(e) = self.engine.delete_instance_by_info_hash(info_hash).await {
            tracing::warn!("Failed to delete instance by info hash: {}", e);
        }
    }
}

impl<E: WatchEngine + 'static> WatchService<E> {
    pub fn new(config: EngineConfig, engine: Arc<E>) -> Self {
        let auto_start = config.auto_start;
        let max_depth = config.max_depth;
        Self {
            config,
            engine,
            loaded_hashes: Arc::new(RwLock::new(HashSet::new())),
            path_to_hash: Arc::new(RwLock::new(HashMap::new())),
            max_depth: Arc::new(AtomicU32::new(max_depth)),
            auto_start: Arc::new(AtomicBool::new(auto_start)),
            shutdown_tx: None,
        }
    }

    pub fn config(&self) -> EngineConfig {
        self.config.clone()
    }

    pub fn set_max_depth(&mut self, depth: u32) {
        self.config.max_depth = depth;
        self.max_depth.store(depth, Ordering::Relaxed);
    }

    pub fn set_auto_start(&mut self, enabled: bool) {
        self.config.auto_start = enabled;
        self.auto_start.store(enabled, Ordering::Relaxed);
    }

    pub async fn init_from_state(&self) {
        let instances = self.engine.list_instances().await;
        let mut hashes = self.loaded_hashes.write().await;

        let mut watch_folder_to_start = Vec::new();

        for instance in &instances {
            hashes.insert(instance.info_hash);

            if self.auto_start.load(Ordering::Relaxed)
                && instance.source == InstanceSource::WatchFolder
                && !matches!(instance.state.as_str(), "starting" | "running")
            {
                watch_folder_to_start.push(instance.id.clone());
            }
        }

        drop(hashes);

        for id in watch_folder_to_start {
            if let Err(e) = self.engine.start_instance(&id).await {
                tracing::warn!("Failed to auto-start watch folder instance {}: {}", id, e);
            }
        }
    }

    pub async fn start(&mut self) -> Result<(), String> {
        if !self.config.enabled {
            tracing::info!("Watch folder service is disabled");
            return Ok(());
        }

        if !self.config.watch_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&self.config.watch_dir) {
                return Err(format!("Failed to create watch directory: {e}"));
            }
            tracing::info!("Created watch directory: {:?}", self.config.watch_dir);
        }

        self.init_from_state().await;
        self.scan_directory().await;

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let runner = WatchRunner {
            watch_dir: self.config.watch_dir.clone(),
            auto_start: Arc::clone(&self.auto_start),
            max_depth: Arc::clone(&self.max_depth),
            engine: Arc::clone(&self.engine),
            loaded_hashes: Arc::clone(&self.loaded_hashes),
            path_to_hash: Arc::clone(&self.path_to_hash),
            shutdown_rx,
        };

        tokio::spawn(async move {
            if let Err(e) = runner.run().await {
                tracing::error!("Watch service error: {}", e);
            }
        });

        tracing::info!(
            "Watch folder service started: {:?} (auto_start={})",
            self.config.watch_dir,
            self.config.auto_start
        );

        Ok(())
    }

    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
            tracing::info!("Watch folder service stopped");
        }
    }

    async fn scan_directory(&self) {
        let entries = match scan_torrent_paths(&self.config.watch_dir, self.config.max_depth) {
            Ok(entries) => entries,
            Err(e) => {
                tracing::warn!("Failed to scan watch directory: {}", e);
                return;
            }
        };

        let context = WatchContext {
            auto_start: &self.auto_start,
            engine: &self.engine,
            loaded_hashes: &self.loaded_hashes,
            path_to_hash: &self.path_to_hash,
            watch_dir: &self.config.watch_dir,
        };

        for path in entries {
            if let Err(e) = process_torrent_file(&path, &context).await {
                tracing::warn!("Failed to process {:?}: {}", path, e);
            }
        }
    }

    pub async fn get_status(&self) -> WatchStatus {
        let loaded_count = self.loaded_hashes.read().await.len();
        let file_count = scan_torrent_paths(&self.config.watch_dir, self.config.max_depth)
            .map(|entries| entries.len())
            .unwrap_or(0);

        WatchStatus {
            enabled: self.config.enabled,
            watch_dir: self.config.watch_dir.to_string_lossy().to_string(),
            auto_start: self.auto_start.load(Ordering::Relaxed),
            file_count,
            loaded_count,
        }
    }

    pub async fn list_files(&self) -> Vec<WatchedFile> {
        let mut files = Vec::new();
        let loaded_hashes = self.loaded_hashes.read().await;

        let Ok(entries) = scan_torrent_paths(&self.config.watch_dir, self.config.max_depth) else {
            return files;
        };

        for path in entries {
            let Ok(relative) = relative_watch_path(&self.config.watch_dir, &path) else {
                continue;
            };

            let filename = relative.to_string_lossy().to_string();
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

            let (status, info_hash, name) =
                std::fs::read(&path).map_or((WatchedFileStatus::Invalid, None, None), |data| {
                    match TorrentSummary::from_bytes(&data) {
                        Ok(torrent) => {
                            let hash = torrent.info_hash;
                            let hash_hex = hex::encode(hash);

                            let status = if loaded_hashes.contains(&hash) {
                                WatchedFileStatus::Loaded
                            } else {
                                WatchedFileStatus::Pending
                            };

                            (status, Some(hash_hex), Some(torrent.name))
                        }
                        Err(_) => (WatchedFileStatus::Invalid, None, None),
                    }
                });

            files.push(WatchedFile {
                filename,
                path: path.to_string_lossy().to_string(),
                status,
                info_hash,
                name,
                size,
            });
        }

        files.sort_by(|a, b| a.filename.cmp(&b.filename));
        files
    }

    pub async fn reload_file(&self, filename: &str) -> Result<(), String> {
        let (canonical_file, relative) = resolve_watch_file(&self.config.watch_dir, filename)?;
        let detach = HashDetachCtx {
            engine: &self.engine,
            loaded_hashes: &self.loaded_hashes,
            path_to_hash: &self.path_to_hash,
        };

        let info_hash = {
            let path_to_hash = self.path_to_hash.read().await;
            path_to_hash.get(&relative).copied()
        };

        if let Some(hash) = info_hash {
            detach.detach(Some(&relative), &hash, false).await;
        }

        let context = WatchContext {
            auto_start: &self.auto_start,
            engine: &self.engine,
            loaded_hashes: &self.loaded_hashes,
            path_to_hash: &self.path_to_hash,
            watch_dir: &self.config.watch_dir,
        };

        process_torrent_file(&canonical_file, &context).await?;

        tracing::info!("Reloaded torrent file: {}", filename);
        Ok(())
    }

    pub async fn reload_all(&self) -> Result<u32, String> {
        if !self.config.watch_dir.exists() {
            return Err("Watch directory does not exist".to_string());
        }

        let entries = scan_torrent_paths(&self.config.watch_dir, self.config.max_depth)
            .map_err(|e| format!("Failed to read watch directory: {e}"))?;

        let context = WatchContext {
            auto_start: &self.auto_start,
            engine: &self.engine,
            loaded_hashes: &self.loaded_hashes,
            path_to_hash: &self.path_to_hash,
            watch_dir: &self.config.watch_dir,
        };
        let detach = HashDetachCtx {
            engine: &self.engine,
            loaded_hashes: &self.loaded_hashes,
            path_to_hash: &self.path_to_hash,
        };

        let mut count = 0;
        for path in entries {
            let Ok(data) = std::fs::read(&path) else {
                continue;
            };

            let Ok(torrent) = TorrentSummary::from_bytes(&data) else {
                continue;
            };

            let info_hash = torrent.info_hash;
            let already_loaded = self.loaded_hashes.read().await.contains(&info_hash);

            if already_loaded {
                let relative = relative_watch_path(&self.config.watch_dir, &path).ok();
                detach.detach(relative.as_ref(), &info_hash, false).await;
            }

            if let Err(e) = process_torrent_file(&path, &context).await {
                tracing::warn!("Failed to process {:?}: {}", path, e);
            } else {
                count += 1;
            }
        }

        if count > 0 {
            tracing::info!("Reloaded {} torrent(s) from watch folder", count);
        }

        Ok(count)
    }

    pub async fn delete_file(&self, filename: &str) -> Result<(), String> {
        let (canonical_file, relative) = resolve_watch_file(&self.config.watch_dir, filename)?;
        let detach = HashDetachCtx {
            engine: &self.engine,
            loaded_hashes: &self.loaded_hashes,
            path_to_hash: &self.path_to_hash,
        };

        let info_hash = {
            let path_to_hash = self.path_to_hash.read().await;
            path_to_hash.get(&relative).copied()
        };

        std::fs::remove_file(&canonical_file).map_err(|e| format!("Failed to delete file: {e}"))?;

        if let Some(hash) = info_hash {
            detach.detach(Some(&relative), &hash, true).await;
        }

        Ok(())
    }

    pub async fn remove_info_hash(&self, info_hash: &[u8; 20]) {
        let removed = self.loaded_hashes.write().await.remove(info_hash);
        if removed {
            tracing::info!("Removed info_hash {} from watch service", hex::encode(info_hash));
        }
    }
}

struct WatchContext<'a, E: WatchEngine> {
    auto_start: &'a Arc<AtomicBool>,
    engine: &'a Arc<E>,
    loaded_hashes: &'a Arc<RwLock<HashSet<[u8; 20]>>>,
    path_to_hash: &'a Arc<RwLock<HashMap<PathBuf, [u8; 20]>>>,
    watch_dir: &'a Path,
}

async fn process_torrent_file<E: WatchEngine>(
    path: &Path,
    context: &WatchContext<'_, E>,
) -> Result<(), String> {
    let auto_start = context.auto_start.load(Ordering::Relaxed);
    let data = std::fs::read(path).map_err(|e| format!("Failed to read torrent file: {e}"))?;

    let torrent =
        TorrentSummary::from_bytes(&data).map_err(|e| format!("Failed to parse torrent: {e}"))?;

    let info_hash = torrent.info_hash;

    {
        let hashes = context.loaded_hashes.read().await;
        if hashes.contains(&info_hash) {
            if context.engine.find_instance_by_info_hash(&info_hash).await.is_some() {
                if let Err(e) = context
                    .engine
                    .update_instance_source_by_info_hash(&info_hash, InstanceSource::WatchFolder)
                    .await
                {
                    tracing::warn!("Failed to update instance source: {}", e);
                }
            }

            let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            if let Ok(relative) = relative_watch_path(context.watch_dir, &canonical_path) {
                context.path_to_hash.write().await.insert(relative, info_hash);
            }

            tracing::warn!(
                "Skipping duplicate torrent '{}' (info_hash: {})",
                torrent.name,
                hex::encode(info_hash)
            );
            return Ok(());
        }
    }

    let new_id = context.engine.next_instance_id();
    let config =
        context.engine.default_config().await.unwrap_or_else(rustatio_core::FakerConfig::default);
    let instance = NewInstance { id: new_id.clone(), info: torrent.to_info(), config, auto_start };

    context.engine.create_instance(instance).await?;

    context.loaded_hashes.write().await.insert(info_hash);

    let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let relative = relative_watch_path(context.watch_dir, &canonical_path)?;
    context.path_to_hash.write().await.insert(relative, info_hash);

    tracing::info!("Loaded torrent '{}' from watch folder as instance", torrent.name);

    if auto_start {
        if let Err(e) = context.engine.start_instance(&new_id).await {
            tracing::warn!("Failed to auto-start instance {}: {}", new_id, e);
        }
    }

    Ok(())
}

pub struct WatchRunner<E: WatchEngine> {
    watch_dir: PathBuf,
    auto_start: Arc<AtomicBool>,
    max_depth: Arc<AtomicU32>,
    engine: Arc<E>,
    loaded_hashes: Arc<RwLock<HashSet<[u8; 20]>>>,
    path_to_hash: Arc<RwLock<HashMap<PathBuf, [u8; 20]>>>,
    shutdown_rx: mpsc::Receiver<()>,
}

impl<E: WatchEngine> WatchRunner<E> {
    async fn run(mut self) -> Result<(), String> {
        let (tx, mut rx) = mpsc::channel(100);

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.blocking_send(event);
                }
            },
            Config::default(),
        )
        .map_err(|e| format!("Failed to create watcher: {e}"))?;

        watcher
            .watch(&self.watch_dir, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch directory: {e}"))?;

        loop {
            tokio::select! {
                _ = self.shutdown_rx.recv() => {
                    break;
                }
                Some(event) = rx.recv() => {
                    if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                        for path in event.paths {
                            let max_depth = self.max_depth.load(Ordering::Relaxed);
                            if is_torrent_file(&path)
                                && crate::paths::is_within_depth(&self.watch_dir, &path, max_depth, false)
                            {
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                                let context = WatchContext {
                                    auto_start: &self.auto_start,
                                    engine: &self.engine,
                                    loaded_hashes: &self.loaded_hashes,
                                    path_to_hash: &self.path_to_hash,
                                    watch_dir: &self.watch_dir,
                                };

                                if let Err(e) = process_torrent_file(&path, &context).await {
                                    tracing::warn!("Failed to process {:?}: {}", path, e);
                                }
                            }
                        }
                    } else if matches!(event.kind, EventKind::Remove(_)) {
                        let detach = HashDetachCtx {
                            engine: &self.engine,
                            loaded_hashes: &self.loaded_hashes,
                            path_to_hash: &self.path_to_hash,
                        };

                        for path in event.paths {
                            let (info_hash, matched_path) = {
                                let mapping = self.path_to_hash.read().await;
                                let relative = relative_watch_path(&self.watch_dir, &path).ok();

                                relative.map_or((None, None), |relative| {
                                    mapping.get(&relative).map_or((None, None), |&hash| {
                                        (Some(hash), Some(relative.clone()))
                                    })
                                })
                            };

                            if let Some(hash) = info_hash {
                                detach.detach(matched_path.as_ref(), &hash, true).await;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[derive(Clone, Default)]
    struct MockEngine {
        instances: Arc<RwLock<Vec<InstanceState>>>,
        started: Arc<RwLock<Vec<String>>>,
    }

    impl MockEngine {
        fn with_instances(instances: Vec<InstanceState>) -> Self {
            Self {
                instances: Arc::new(RwLock::new(instances)),
                started: Arc::new(RwLock::new(Vec::new())),
            }
        }

        async fn started_ids(&self) -> Vec<String> {
            self.started.read().await.clone()
        }
    }

    #[async_trait]
    impl WatchEngine for MockEngine {
        async fn list_instances(&self) -> Vec<InstanceState> {
            self.instances.read().await.clone()
        }

        async fn create_instance(&self, _instance: NewInstance) -> Result<(), String> {
            Ok(())
        }

        async fn start_instance(&self, id: &str) -> Result<(), String> {
            self.started.write().await.push(id.to_string());
            Ok(())
        }

        async fn delete_instance_by_info_hash(&self, _info_hash: &[u8; 20]) -> Result<(), String> {
            Ok(())
        }

        async fn find_instance_by_info_hash(&self, info_hash: &[u8; 20]) -> Option<String> {
            self.instances
                .read()
                .await
                .iter()
                .find(|inst| inst.info_hash == *info_hash)
                .map(|inst| inst.id.clone())
        }

        async fn update_instance_source_by_info_hash(
            &self,
            info_hash: &[u8; 20],
            source: InstanceSource,
        ) -> Result<(), String> {
            let mut instances = self.instances.write().await;
            for inst in instances.iter_mut() {
                if inst.info_hash == *info_hash {
                    inst.source = source;
                }
            }
            Ok(())
        }

        async fn default_config(&self) -> Option<rustatio_core::FakerConfig> {
            None
        }

        fn next_instance_id(&self) -> String {
            "1".to_string()
        }
    }

    fn watch_instance(id: &str, state: &str) -> InstanceState {
        InstanceState {
            id: id.to_string(),
            info_hash: [1; 20],
            source: InstanceSource::WatchFolder,
            state: state.to_string(),
            name: "watch".to_string(),
        }
    }

    #[tokio::test]
    async fn init_from_state_starts_stopped_watch_instances_when_auto_start_enabled(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let engine = MockEngine::with_instances(vec![watch_instance("w1", "stopped")]);
        let config = EngineConfig {
            watch_dir: temp.path().to_path_buf(),
            auto_start: true,
            enabled: true,
            max_depth: 1,
        };

        let service = WatchService::new(config, Arc::new(engine.clone()));
        service.init_from_state().await;

        let started = engine.started_ids().await;
        assert_eq!(started, vec!["w1".to_string()]);
        Ok(())
    }

    #[tokio::test]
    async fn init_from_state_does_not_restart_running_watch_instances(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let engine = MockEngine::with_instances(vec![watch_instance("w1", "running")]);
        let config = EngineConfig {
            watch_dir: temp.path().to_path_buf(),
            auto_start: true,
            enabled: true,
            max_depth: 1,
        };

        let service = WatchService::new(config, Arc::new(engine.clone()));
        service.init_from_state().await;

        let started = engine.started_ids().await;
        assert!(started.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn init_from_state_does_not_start_when_auto_start_is_disabled(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let engine = MockEngine::with_instances(vec![watch_instance("w1", "stopped")]);
        let config = EngineConfig {
            watch_dir: temp.path().to_path_buf(),
            auto_start: false,
            enabled: true,
            max_depth: 1,
        };

        let service = WatchService::new(config, Arc::new(engine.clone()));
        service.init_from_state().await;

        let started = engine.started_ids().await;
        assert!(started.is_empty());
        Ok(())
    }
}
