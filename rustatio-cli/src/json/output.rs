use chrono::{DateTime, Utc};
use rustatio_core::{ClientType, FakerState, FakerStats, TorrentInfo};
use serde::Serialize;

/// All JSON output events
#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum OutputEvent {
    /// Initial event with CLI version
    Init(InitEvent),

    /// Torrent file loaded successfully
    TorrentLoaded(TorrentLoadedEvent),

    /// Faker started successfully
    Started(StartedEvent),

    /// Tracker announce completed
    Announce(AnnounceEvent),

    /// Periodic stats update
    Stats(StatsEvent),

    /// Faker paused
    Paused(PausedEvent),

    /// Faker resumed
    Resumed(ResumedEvent),

    /// Scrape response
    Scrape(ScrapeEvent),

    /// Faker stopped
    Stopped(StoppedEvent),

    /// Error occurred
    Error(ErrorEvent),
}

#[derive(Debug, Serialize)]
pub struct InitEvent {
    pub version: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TorrentLoadedEvent {
    pub name: String,
    pub size: u64,
    pub info_hash: String,
    pub tracker: String,
    pub num_pieces: usize,
    pub piece_length: u64,
    pub is_single_file: bool,
    pub file_count: usize,
    pub timestamp: DateTime<Utc>,
}

impl From<&TorrentInfo> for TorrentLoadedEvent {
    fn from(torrent: &TorrentInfo) -> Self {
        TorrentLoadedEvent {
            name: torrent.name.clone(),
            size: torrent.total_size,
            info_hash: torrent.info_hash_hex(),
            tracker: torrent.announce.clone(),
            num_pieces: torrent.num_pieces,
            piece_length: torrent.piece_length,
            is_single_file: torrent.is_single_file,
            file_count: torrent.files.len(),
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StartedEvent {
    pub peer_id: String,
    pub client: String,
    pub client_version: String,
    pub port: u16,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AnnounceEvent {
    #[serde(rename = "type")]
    pub announce_type: AnnounceType,
    pub seeders: i64,
    pub leechers: i64,
    pub interval: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum AnnounceType {
    Started,
    Periodic,
    Stopped,
}

#[derive(Debug, Serialize)]
pub struct StatsEvent {
    // Transfer stats
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,

    // Ratios
    pub ratio: f64,
    pub session_ratio: f64,

    // Session stats
    pub session_uploaded: u64,
    pub session_downloaded: u64,

    // Rates (KB/s)
    pub upload_rate: f64,
    pub download_rate: f64,
    pub avg_upload_rate: f64,
    pub avg_download_rate: f64,

    // Tracker info
    pub seeders: i64,
    pub leechers: i64,

    // Progress
    pub upload_progress: f64,
    pub download_progress: f64,
    pub ratio_progress: f64,
    pub seed_time_progress: f64,

    // ETA (seconds, null if not applicable)
    pub eta_ratio_secs: Option<u64>,
    pub eta_uploaded_secs: Option<u64>,
    pub eta_seed_time_secs: Option<u64>,

    // State
    pub state: String,
    pub elapsed_secs: u64,

    pub timestamp: DateTime<Utc>,
}

impl From<&FakerStats> for StatsEvent {
    fn from(stats: &FakerStats) -> Self {
        StatsEvent {
            uploaded: stats.uploaded,
            downloaded: stats.downloaded,
            left: stats.left,
            ratio: stats.ratio,
            session_ratio: stats.session_ratio,
            session_uploaded: stats.session_uploaded,
            session_downloaded: stats.session_downloaded,
            upload_rate: stats.current_upload_rate,
            download_rate: stats.current_download_rate,
            avg_upload_rate: stats.average_upload_rate,
            avg_download_rate: stats.average_download_rate,
            seeders: stats.seeders,
            leechers: stats.leechers,
            upload_progress: stats.upload_progress,
            download_progress: stats.download_progress,
            ratio_progress: stats.ratio_progress,
            seed_time_progress: stats.seed_time_progress,
            eta_ratio_secs: stats.eta_ratio.map(|d| d.as_secs()),
            eta_uploaded_secs: stats.eta_uploaded.map(|d| d.as_secs()),
            eta_seed_time_secs: stats.eta_seed_time.map(|d| d.as_secs()),
            state: format_state(&stats.state),
            elapsed_secs: stats.elapsed_time.as_secs(),
            timestamp: Utc::now(),
        }
    }
}

fn format_state(state: &FakerState) -> String {
    match state {
        FakerState::Idle => "idle".to_string(),
        FakerState::Starting => "starting".to_string(),
        FakerState::Running => "running".to_string(),
        FakerState::Stopping => "stopping".to_string(),
        FakerState::Paused => "paused".to_string(),
        FakerState::Stopped => "stopped".to_string(),
    }
}

#[derive(Debug, Serialize)]
pub struct PausedEvent {
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ResumedEvent {
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ScrapeEvent {
    pub seeders: i64,
    pub leechers: i64,
    pub downloaded: i64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct StoppedEvent {
    pub reason: StopReason,
    pub final_uploaded: u64,
    pub final_downloaded: u64,
    pub final_ratio: f64,
    pub session_uploaded: u64,
    pub session_ratio: f64,
    pub elapsed_secs: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum StopReason {
    UserCommand,
    UserInterrupt,
    TargetRatio,
    TargetUploaded,
    TargetDownloaded,
    TargetSeedTime,
    NoLeechers,
    Error,
}

#[derive(Debug, Serialize)]
pub struct ErrorEvent {
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

impl OutputEvent {
    /// Serialize event to JSON and print to stdout
    pub fn emit(&self) {
        if let Ok(json) = serde_json::to_string(self) {
            println!("{}", json);
        }
    }

    /// Helper to emit init event
    pub fn init() -> Self {
        OutputEvent::Init(InitEvent {
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: Utc::now(),
        })
    }

    /// Helper to emit error event
    pub fn error(message: impl Into<String>) -> Self {
        OutputEvent::Error(ErrorEvent {
            message: message.into(),
            timestamp: Utc::now(),
        })
    }

    /// Helper to emit paused event
    pub fn paused() -> Self {
        OutputEvent::Paused(PausedEvent { timestamp: Utc::now() })
    }

    /// Helper to emit resumed event
    pub fn resumed() -> Self {
        OutputEvent::Resumed(ResumedEvent { timestamp: Utc::now() })
    }
}

/// Output for the `info` subcommand
#[derive(Debug, Serialize)]
pub struct TorrentInfoOutput {
    pub name: String,
    pub size: u64,
    pub size_human: String,
    pub info_hash: String,
    pub tracker: String,
    pub trackers: Vec<String>,
    pub num_pieces: usize,
    pub piece_length: u64,
    pub piece_length_human: String,
    pub is_single_file: bool,
    pub files: Vec<FileOutput>,
    pub creation_date: Option<String>,
    pub created_by: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileOutput {
    pub path: String,
    pub size: u64,
    pub size_human: String,
}

impl From<&TorrentInfo> for TorrentInfoOutput {
    fn from(torrent: &TorrentInfo) -> Self {
        TorrentInfoOutput {
            name: torrent.name.clone(),
            size: torrent.total_size,
            size_human: format_bytes(torrent.total_size),
            info_hash: torrent.info_hash_hex(),
            tracker: torrent.announce.clone(),
            trackers: torrent.get_all_tracker_urls(),
            num_pieces: torrent.num_pieces,
            piece_length: torrent.piece_length,
            piece_length_human: format_bytes(torrent.piece_length),
            is_single_file: torrent.is_single_file,
            files: torrent
                .files
                .iter()
                .map(|f| FileOutput {
                    path: f.path.join("/"),
                    size: f.length,
                    size_human: format_bytes(f.length),
                })
                .collect(),
            creation_date: torrent.creation_date.map(|ts| {
                DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| ts.to_string())
            }),
            created_by: torrent.created_by.clone(),
            comment: torrent.comment.clone(),
        }
    }
}

/// Output for the `clients` subcommand
#[derive(Debug, Serialize)]
pub struct ClientsOutput {
    pub clients: Vec<ClientInfoOutput>,
}

/// Client info output (local struct to control serialization)
#[derive(Debug, Serialize)]
pub struct ClientInfoOutput {
    pub id: String,
    pub name: String,
    pub default_version: String,
    pub versions: Vec<String>,
    pub default_port: u16,
}

impl From<rustatio_core::ClientInfo> for ClientInfoOutput {
    fn from(info: rustatio_core::ClientInfo) -> Self {
        ClientInfoOutput {
            id: info.id,
            name: info.name,
            default_version: info.default_version,
            versions: info.versions,
            default_port: info.default_port,
        }
    }
}

impl ClientsOutput {
    pub fn new() -> Self {
        ClientsOutput {
            clients: ClientType::all_infos().into_iter().map(|i| i.into()).collect(),
        }
    }
}

impl Default for ClientsOutput {
    fn default() -> Self {
        Self::new()
    }
}

/// Format bytes to human readable string
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format duration to human readable string
pub fn format_duration(secs: u64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    if hours > 0 {
        format!("{}h {:02}m {:02}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {:02}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}
