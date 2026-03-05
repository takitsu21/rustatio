use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub watch_dir: PathBuf,
    pub auto_start: bool,
    pub enabled: bool,
    pub max_depth: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct WatchedFile {
    pub filename: String,
    pub path: String,
    pub status: WatchedFileStatus,
    pub info_hash: Option<String>,
    pub name: Option<String>,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WatchedFileStatus {
    Pending,
    Loaded,
    Invalid,
}

#[derive(Debug, Clone, Serialize)]
pub struct WatchStatus {
    pub enabled: bool,
    pub watch_dir: String,
    pub auto_start: bool,
    pub file_count: usize,
    pub loaded_count: usize,
}
