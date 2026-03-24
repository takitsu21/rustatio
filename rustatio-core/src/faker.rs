use crate::protocol::{
    AnnounceRequest, AnnounceResponse, TrackerClient, TrackerError, TrackerEvent,
};
use crate::torrent::{ClientConfig, ClientType, TorrentInfo};
use crate::{log_debug, log_info, log_trace, log_warn};
#[cfg(not(target_arch = "wasm32"))]
use crate::{peer_listener::handle_is_connectable, protocol::peer_id_to_array};
use instant::Instant;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::{watch, Mutex};

#[cfg(target_arch = "wasm32")]
use js_sys;

#[derive(Debug, Error)]
pub enum FakerError {
    #[error("Tracker error: {0}")]
    TrackerError(#[from] TrackerError),
    #[error("Invalid state: {0}")]
    InvalidState(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, FakerError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FakerConfig {
    /// Upload rate in KB/s
    pub upload_rate: f64,

    /// Download rate in KB/s
    pub download_rate: f64,

    /// Port to announce
    pub port: u16,

    /// Sync announced port from the VPN forwarded port when available
    #[serde(default)]
    pub vpn_port_sync: bool,

    /// Client to emulate
    pub client_type: ClientType,

    /// Client version (optional, uses default if None)
    pub client_version: Option<String>,

    /// Initial uploaded amount in bytes
    pub initial_uploaded: u64,

    /// Initial downloaded amount in bytes
    pub initial_downloaded: u64,

    /// Percentage already downloaded (0-100)
    pub completion_percent: f64,

    /// Number of peers to request
    pub num_want: u32,

    /// Enable randomization of rates
    #[serde(default = "default_randomize_rates")]
    pub randomize_rates: bool,

    /// Randomization range percentage (e.g., 20 means ±20%)
    #[serde(default = "default_random_range")]
    pub random_range_percent: f64,

    /// Enable randomization of the stop ratio target
    #[serde(default)]
    pub randomize_ratio: bool,

    /// Randomization range percentage for ratio (e.g., 10 means ±10%)
    #[serde(default = "default_random_ratio_range")]
    pub random_ratio_range_percent: f64,

    // Stop conditions
    /// Stop when ratio reaches this value (optional)
    pub stop_at_ratio: Option<f64>,

    /// Pre-computed effective ratio from frontend preview (skips re-randomization if provided)
    #[serde(default)]
    pub effective_stop_at_ratio: Option<f64>,

    /// Stop after uploading this many bytes (optional)
    pub stop_at_uploaded: Option<u64>,

    /// Stop after downloading this many bytes (optional)
    pub stop_at_downloaded: Option<u64>,

    /// Stop after seeding for this many seconds (optional)
    pub stop_at_seed_time: Option<u64>,

    /// Idle (0 KB/s upload) when there are no leechers - stays connected for bonus points (optional, default false)
    #[serde(default)]
    pub idle_when_no_leechers: bool,

    /// Idle (0 KB/s download) when there are no seeders - stays connected for bonus points (optional, default false)
    #[serde(default)]
    pub idle_when_no_seeders: bool,

    /// Interval in seconds between scrape requests for peer count updates (default: 60)
    #[serde(default = "default_scrape_interval")]
    pub scrape_interval: u64,

    // Progressive rate adjustment
    /// Enable progressive rate adjustment
    #[serde(default)]
    pub progressive_rates: bool,

    /// Target upload rate to reach (KB/s)
    pub target_upload_rate: Option<f64>,

    /// Target download rate to reach (KB/s)
    pub target_download_rate: Option<f64>,

    /// Time in seconds to reach target rates
    #[serde(default = "default_progressive_duration")]
    pub progressive_duration: u64,

    /// What to do when stop conditions are met
    #[serde(default)]
    pub post_stop_action: PostStopAction,
}

/// UI-friendly preset settings format (matches frontend)
/// Uses human-readable units (GB, hours) and enabled flags for optional fields
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PresetSettings {
    pub upload_rate: Option<f64>,
    pub download_rate: Option<f64>,
    pub port: Option<u16>,
    pub vpn_port_sync: Option<bool>,
    pub selected_client: Option<ClientType>,
    pub selected_client_version: Option<String>,
    pub completion_percent: Option<f64>,
    pub randomize_rates: Option<bool>,
    pub random_range_percent: Option<f64>,
    pub randomize_ratio: Option<bool>,
    pub random_ratio_range_percent: Option<f64>,
    // Stop conditions with enabled flags
    pub stop_at_ratio_enabled: Option<bool>,
    pub stop_at_ratio: Option<f64>,
    pub stop_at_uploaded_enabled: Option<bool>,
    pub stop_at_uploaded_gb: Option<f64>,
    pub stop_at_downloaded_enabled: Option<bool>,
    pub stop_at_downloaded_gb: Option<f64>,
    pub stop_at_seed_time_enabled: Option<bool>,
    pub stop_at_seed_time_hours: Option<f64>,
    pub idle_when_no_leechers: Option<bool>,
    pub idle_when_no_seeders: Option<bool>,
    pub post_stop_action: Option<String>,
    // Progressive rates
    pub progressive_rates_enabled: Option<bool>,
    pub target_upload_rate: Option<f64>,
    pub target_download_rate: Option<f64>,
    pub progressive_duration_hours: Option<f64>,
}

impl From<PresetSettings> for FakerConfig {
    fn from(p: PresetSettings) -> Self {
        let stop_at_ratio =
            if p.stop_at_ratio_enabled.unwrap_or(false) { p.stop_at_ratio } else { None };

        let stop_at_uploaded = if p.stop_at_uploaded_enabled.unwrap_or(false) {
            p.stop_at_uploaded_gb.map(|gb| (gb * 1024.0 * 1024.0 * 1024.0) as u64)
        } else {
            None
        };

        let stop_at_downloaded = if p.stop_at_downloaded_enabled.unwrap_or(false) {
            p.stop_at_downloaded_gb.map(|gb| (gb * 1024.0 * 1024.0 * 1024.0) as u64)
        } else {
            None
        };

        let stop_at_seed_time = if p.stop_at_seed_time_enabled.unwrap_or(false) {
            p.stop_at_seed_time_hours.map(|h| (h * 3600.0) as u64)
        } else {
            None
        };

        Self {
            upload_rate: p.upload_rate.unwrap_or(50.0),
            download_rate: p.download_rate.unwrap_or(100.0),
            port: p.port.unwrap_or(6881),
            vpn_port_sync: p.vpn_port_sync.unwrap_or(false),
            client_type: p.selected_client.unwrap_or(ClientType::QBittorrent),
            client_version: p.selected_client_version,
            initial_uploaded: 0,
            initial_downloaded: 0,
            completion_percent: p.completion_percent.unwrap_or(100.0),
            num_want: 50,
            randomize_rates: p.randomize_rates.unwrap_or(true),
            random_range_percent: p.random_range_percent.unwrap_or(20.0),
            randomize_ratio: p.randomize_ratio.unwrap_or(false),
            random_ratio_range_percent: p.random_ratio_range_percent.unwrap_or(10.0),
            stop_at_ratio,
            effective_stop_at_ratio: None,
            stop_at_uploaded,
            stop_at_downloaded,
            stop_at_seed_time,
            idle_when_no_leechers: p.idle_when_no_leechers.unwrap_or(false),
            idle_when_no_seeders: p.idle_when_no_seeders.unwrap_or(false),
            scrape_interval: 60,
            post_stop_action: match p.post_stop_action.as_deref() {
                Some("stop_seeding") => PostStopAction::StopSeeding,
                Some("delete_instance") => PostStopAction::DeleteInstance,
                _ => PostStopAction::Idle,
            },
            progressive_rates: p.progressive_rates_enabled.unwrap_or(false),
            target_upload_rate: p.target_upload_rate,
            target_download_rate: p.target_download_rate,
            progressive_duration: (p.progressive_duration_hours.unwrap_or(1.0) * 3600.0) as u64,
        }
    }
}

const fn default_randomize_rates() -> bool {
    true
}

const fn default_progressive_duration() -> u64 {
    3600 // 1 hour
}

const fn default_random_range() -> f64 {
    20.0
}

const fn default_random_ratio_range() -> f64 {
    10.0
}

const fn default_scrape_interval() -> u64 {
    60 // 60 seconds
}

impl Default for FakerConfig {
    fn default() -> Self {
        Self {
            upload_rate: 50.0,    // 50 KB/s
            download_rate: 100.0, // 100 KB/s
            port: 6881,
            vpn_port_sync: false,
            client_type: ClientType::QBittorrent,
            client_version: None,
            initial_uploaded: 0,
            initial_downloaded: 0,
            completion_percent: 0.0,
            num_want: 50,
            randomize_rates: true,
            random_range_percent: 20.0,
            randomize_ratio: false,
            random_ratio_range_percent: 10.0,
            stop_at_ratio: None,
            effective_stop_at_ratio: None,
            stop_at_uploaded: None,
            stop_at_downloaded: None,
            stop_at_seed_time: None,
            idle_when_no_leechers: false,
            idle_when_no_seeders: false,
            scrape_interval: 60,
            progressive_rates: false,
            target_upload_rate: None,
            target_download_rate: None,
            progressive_duration: 3600,
            post_stop_action: PostStopAction::Idle,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostStopAction {
    #[default]
    Idle,
    StopSeeding,
    DeleteInstance,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FakerState {
    Idle,
    Starting,
    Running,
    Stopping,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FakerStats {
    // === CUMULATIVE STATS (lifetime totals for display) ===
    pub uploaded: u64,   // Total uploaded across all sessions
    pub downloaded: u64, // Total downloaded across all sessions
    pub ratio: f64,      // Cumulative ratio: uploaded / torrent_size

    // === TORRENT STATE ===
    pub left: u64,               // Bytes left to download for THIS torrent
    pub torrent_completion: f64, // 0-100% of torrent downloaded
    pub seeders: i64,            // Seeders from tracker
    pub leechers: i64,           // Leechers from tracker
    pub state: FakerState,

    // === IDLE STATE ===
    pub is_idling: bool,               // True when idling due to no peers
    pub idling_reason: Option<String>, // "no_leechers" or "no_seeders"

    // === SESSION STATS (current session only) ===
    pub session_uploaded: u64,   // Uploaded in current session
    pub session_downloaded: u64, // Downloaded in current session
    pub session_ratio: f64,      // Session ratio: session_uploaded / torrent_size
    pub elapsed_time: Duration,  // Time since session started

    // === RATES ===
    pub current_upload_rate: f64,   // Current upload rate KB/s
    pub current_download_rate: f64, // Current download rate KB/s
    pub average_upload_rate: f64,   // Average upload rate KB/s (session)
    pub average_download_rate: f64, // Average download rate KB/s (session)

    // === PROGRESS (session-based for stop conditions) ===
    pub upload_progress: f64,    // 0-100% toward stop_at_uploaded
    pub download_progress: f64,  // 0-100% toward stop_at_downloaded
    pub ratio_progress: f64,     // 0-100% toward stop_at_ratio
    pub seed_time_progress: f64, // 0-100% toward stop_at_seed_time

    // === EFFECTIVE TARGETS (after randomization) ===
    pub effective_stop_at_ratio: Option<f64>, // Actual ratio target used by backend (after randomization)

    // === ETA ===
    pub eta_ratio: Option<Duration>,
    pub eta_uploaded: Option<Duration>,
    pub eta_seed_time: Option<Duration>,
    pub eta_download_completion: Option<Duration>,

    // === HISTORY (for graphs) ===
    pub upload_rate_history: Vec<f64>,
    pub download_rate_history: Vec<f64>,
    pub ratio_history: Vec<f64>,
    pub history_timestamps: Vec<u64>, // Unix timestamps in milliseconds

    // === STOP CONDITION STATE ===
    #[serde(default)]
    pub stop_condition_met: bool,
    #[serde(default)]
    pub post_stop_action: PostStopAction,

    // === INTERNAL ===
    #[serde(skip)]
    pub last_announce: Option<Instant>,
    #[serde(skip)]
    pub next_announce: Option<Instant>,
    pub announce_count: u32,
}

pub struct RatioFaker {
    torrent: Arc<TorrentInfo>,
    config: FakerConfig,
    tracker_client: Arc<TrackerClient>,

    // Runtime state
    stats: FakerStats,

    // Session data
    peer_id: String,
    key: String,
    tracker_id: Option<String>,

    // Timing
    start_time: Instant,
    last_update: Instant,
    announce_interval: Duration,

    // Scrape
    last_scrape: Instant,
    scrape_supported: bool,
}

#[derive(Debug, Clone, Copy)]
struct UpdateOutcome {
    completed: bool,
    stop: bool,
    scrape_due: bool,
    announce_due: bool,
}

struct TickInputs {
    elapsed: Duration,
    elapsed_secs: u64,
    left: u64,
    seeders: i64,
    leechers: i64,
    announce_count: u32,
    torrent_size: u64,
    start_time: Instant,
    config: FakerConfig,
}

struct AnnouncePlan {
    tracker_client: Arc<TrackerClient>,
    tracker_url: String,
    request: AnnounceRequest,
}

impl AnnouncePlan {
    async fn execute(&self) -> Result<AnnounceResponse> {
        self.tracker_client
            .announce(&self.tracker_url, &self.request)
            .await
            .map_err(FakerError::from)
    }
}

struct ScrapePlan {
    tracker_client: Arc<TrackerClient>,
    tracker_url: String,
    info_hash: [u8; 20],
}

impl ScrapePlan {
    async fn execute(&self) -> Result<crate::protocol::ScrapeResponse> {
        self.tracker_client
            .scrape(&self.tracker_url, &self.info_hash)
            .await
            .map_err(FakerError::from)
    }
}

impl RatioFaker {
    fn resolve_stop_ratio(config: &mut FakerConfig) {
        if config.randomize_ratio {
            if let Some(base_ratio) = config.stop_at_ratio {
                let effective = if let Some(precomputed) = config.effective_stop_at_ratio {
                    log_info!(
                        "Using pre-computed stop ratio: base={:.4}, effective={:.4}",
                        base_ratio,
                        precomputed
                    );
                    precomputed
                } else {
                    let range = config.random_ratio_range_percent.clamp(0.0, 100.0) / 100.0;
                    let mut rng = rand::rng();
                    let variation: f64 = rng.random::<f64>().mul_add(2.0, -1.0).mul_add(range, 1.0);
                    let computed = (base_ratio * variation * 10000.0).round() / 10000.0;
                    log_info!(
                        "Randomized stop ratio: base={:.4}, range=±{:.0}%, effective={:.4}",
                        base_ratio,
                        config.random_ratio_range_percent,
                        computed
                    );
                    computed
                };
                config.stop_at_ratio = Some(effective);
            }
        }
    }

    /// Create a new `RatioFaker`.
    ///
    /// * `torrent` — shared torrent metadata (`Arc` avoids duplicating large data per instance).
    /// * `config` — faker configuration (rates, stop conditions, etc.).
    /// * `http_client` — if `Some`, reuses the provided `reqwest::Client`.
    pub fn new(
        torrent: Arc<TorrentInfo>,
        config: FakerConfig,
        http_client: Option<reqwest::Client>,
    ) -> Result<Self> {
        log_debug!(
            "Creating RatioFaker for '{}' (size: {} bytes)",
            torrent.name,
            torrent.total_size
        );
        log_trace!(
            "Config: upload_rate={} KB/s, download_rate={} KB/s, client={:?}",
            config.upload_rate,
            config.download_rate,
            config.client_type
        );

        // Create client configuration
        let client_config = ClientConfig::get(config.client_type, config.client_version.clone());

        // Generate session identifiers
        let peer_id = client_config.generate_peer_id();
        let key = ClientConfig::generate_key();

        log_trace!("Generated peer_id: {}, key: {}", peer_id, key);

        // Create tracker client
        let tracker_client = TrackerClient::new(client_config, http_client)
            .map_err(|e| FakerError::ConfigError(e.to_string()))?;

        let mut config = config;
        Self::resolve_stop_ratio(&mut config);

        // Calculate how much of THIS torrent is already downloaded
        let completion = config.completion_percent.clamp(0.0, 100.0) / 100.0;
        let torrent_downloaded = (torrent.total_size as f64 * completion) as u64;
        let left = torrent.total_size.saturating_sub(torrent_downloaded);

        let stats = FakerStats {
            // Cumulative stats from previous sessions
            uploaded: config.initial_uploaded,
            downloaded: config.initial_downloaded,
            ratio: if config.initial_downloaded > 0 {
                config.initial_uploaded as f64 / config.initial_downloaded as f64
            } else {
                0.0
            },

            // Torrent state
            left,
            torrent_completion: if torrent.total_size > 0 {
                ((torrent.total_size - left) as f64 / torrent.total_size as f64) * 100.0
            } else {
                100.0
            },
            seeders: 0,
            leechers: 0,
            state: FakerState::Stopped,

            // Idle state
            is_idling: false,
            idling_reason: None,

            // Session stats (starts fresh at 0)
            session_uploaded: 0,
            session_downloaded: 0,
            session_ratio: 0.0,
            elapsed_time: Duration::from_secs(0),

            // Rates
            current_upload_rate: 0.0,
            current_download_rate: 0.0,
            average_upload_rate: 0.0,
            average_download_rate: 0.0,

            // Progress
            upload_progress: 0.0,
            download_progress: 0.0,
            ratio_progress: 0.0,
            seed_time_progress: 0.0,

            // Effective targets
            effective_stop_at_ratio: config.stop_at_ratio,

            // ETA
            eta_ratio: None,
            eta_uploaded: None,
            eta_seed_time: None,
            eta_download_completion: None,

            // History
            upload_rate_history: Vec::new(),
            download_rate_history: Vec::new(),
            ratio_history: Vec::new(),
            history_timestamps: Vec::new(),

            // Internal
            last_announce: None,
            next_announce: None,
            announce_count: 0,

            stop_condition_met: false,
            post_stop_action: config.post_stop_action,
        };

        Ok(Self {
            torrent,
            config,
            tracker_client: Arc::new(tracker_client),
            stats,
            peer_id,
            key,
            tracker_id: None,
            start_time: Instant::now(),
            last_update: Instant::now(),
            announce_interval: Duration::from_secs(1800), // Default 30 minutes
            last_scrape: Instant::now(),
            scrape_supported: true,
        })
    }

    /// Start the ratio faking session
    pub async fn start(&mut self) -> Result<()> {
        if let Some(plan) = self.begin_start() {
            let result = plan.execute().await;
            self.apply_start_result(result);
        }
        Ok(())
    }

    fn begin_start(&mut self) -> Option<AnnouncePlan> {
        if matches!(self.stats.state, FakerState::Running | FakerState::Starting) {
            return None;
        }

        log_info!("Starting ratio faker for torrent: {}", self.torrent.name);

        self.reset_session_state_for_start();
        self.start_time = Instant::now();
        self.last_update = Instant::now();

        let request = self.build_announce_request(TrackerEvent::Started);

        Some(AnnouncePlan {
            tracker_client: Arc::clone(&self.tracker_client),
            tracker_url: self.torrent.get_tracker_url().to_string(),
            request,
        })
    }

    fn reset_session_state_for_start(&mut self) {
        self.stats.session_uploaded = 0;
        self.stats.session_downloaded = 0;
        self.stats.session_ratio = 0.0;
        self.stats.elapsed_time = Duration::from_secs(0);
        self.stats.current_upload_rate = 0.0;
        self.stats.current_download_rate = 0.0;
        self.stats.average_upload_rate = 0.0;
        self.stats.average_download_rate = 0.0;
        self.stats.upload_progress = 0.0;
        self.stats.download_progress = 0.0;
        self.stats.ratio_progress = 0.0;
        self.stats.seed_time_progress = 0.0;
        self.stats.eta_ratio = None;
        self.stats.eta_uploaded = None;
        self.stats.eta_seed_time = None;
        self.stats.eta_download_completion = None;
        self.stats.upload_rate_history.clear();
        self.stats.download_rate_history.clear();
        self.stats.ratio_history.clear();
        self.stats.history_timestamps.clear();
        self.stats.last_announce = None;
        self.stats.next_announce = None;
        self.stats.announce_count = 0;
        self.stats.stop_condition_met = false;
        self.stats.is_idling = false;
        self.stats.idling_reason = None;
        self.stats.state = FakerState::Starting;
    }

    fn rebase_timers_from_elapsed(&mut self, now: Instant) {
        self.start_time = now.checked_sub(self.stats.elapsed_time).unwrap_or(now);
        self.last_update = now;
    }

    pub fn restore_runtime(&mut self, mut stats: FakerStats) {
        let now = Instant::now();

        stats.post_stop_action = self.config.post_stop_action;
        stats.last_announce = None;
        stats.next_announce = None;

        if matches!(stats.state, FakerState::Starting) {
            stats.state = FakerState::Running;
        }

        self.stats = stats;
        self.rebase_timers_from_elapsed(now);
        self.tracker_id = None;
        self.announce_interval = Duration::from_secs(1800);
        self.last_scrape = now;
        self.scrape_supported = true;
    }

    fn begin_restore_running(&mut self) -> AnnouncePlan {
        log_info!("Restoring active ratio faker for torrent: {}", self.torrent.name);

        self.rebase_timers_from_elapsed(Instant::now());
        self.stats.state = FakerState::Running;

        AnnouncePlan {
            tracker_client: Arc::clone(&self.tracker_client),
            tracker_url: self.torrent.get_tracker_url().to_string(),
            request: self.build_announce_request(TrackerEvent::Started),
        }
    }

    fn apply_start_result(&mut self, result: Result<AnnounceResponse>) {
        match result {
            Ok(response) => {
                self.announce_interval = Duration::from_secs(response.interval as u64);
                self.tracker_id = response.tracker_id;

                self.stats.seeders = response.complete;
                self.stats.leechers = response.incomplete;
                self.stats.last_announce = Some(Instant::now());
                self.stats.next_announce = Some(Instant::now() + self.announce_interval);
                self.stats.announce_count += 1;

                log_info!(
                    "Started successfully. Seeders: {}, Leechers: {}, Interval: {}s",
                    response.complete,
                    response.incomplete,
                    response.interval
                );
            }
            Err(e) => {
                log_warn!("Initial announce failed, will retry on next cycle: {}", e);
                self.stats.next_announce = Some(Instant::now() + Duration::from_secs(30));
            }
        }

        self.stats.state = FakerState::Running;
    }

    /// Stop the ratio faking session
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(plan) = self.begin_stop() {
            let result = plan.execute().await;
            self.apply_stop_result(result);
        }
        Ok(())
    }

    fn begin_stop(&mut self) -> Option<AnnouncePlan> {
        if matches!(self.stats.state, FakerState::Stopped) {
            log_debug!("Already stopped, skipping stop");
            return None;
        }

        log_info!("Stopping ratio faker");

        self.stats.state = FakerState::Stopping;

        Some(AnnouncePlan {
            tracker_client: Arc::clone(&self.tracker_client),
            tracker_url: self.torrent.get_tracker_url().to_string(),
            request: self.build_announce_request(TrackerEvent::Stopped),
        })
    }

    fn apply_stop_result(&mut self, result: Result<AnnounceResponse>) {
        match result {
            Ok(_) => {
                self.stats.announce_count += 1;
                log_info!("Stop announce sent successfully");
            }
            Err(e) => {
                log_warn!("Stop announce failed (tracker will time out peer): {}", e);
            }
        }

        self.stats.state = FakerState::Stopped;
        self.stats.is_idling = false;
        self.stats.idling_reason = None;
    }

    async fn apply_post_stop_action(&mut self) -> Result<()> {
        self.stats.stop_condition_met = true;
        match self.config.post_stop_action {
            PostStopAction::Idle => {
                log_info!("Stop condition met, idling (post_stop_action=idle)");
                self.stats.is_idling = true;
                self.stats.idling_reason = Some("stop_condition_met".to_string());
                self.stats.current_upload_rate = 0.0;
                self.stats.current_download_rate = 0.0;
            }
            PostStopAction::StopSeeding | PostStopAction::DeleteInstance => {
                log_info!("Stop condition met, stopping faker");
                self.stop().await?;
            }
        }
        Ok(())
    }

    /// Update the fake stats (call this periodically)
    pub async fn update(&mut self) -> Result<()> {
        let now = Instant::now();
        let outcome = self.tick(now);

        if outcome.completed {
            let plan = AnnouncePlan {
                tracker_client: Arc::clone(&self.tracker_client),
                tracker_url: self.torrent.get_tracker_url().to_string(),
                request: self.build_announce_request(TrackerEvent::Completed),
            };
            match plan.execute().await {
                Ok(response) => {
                    self.stats.seeders = response.complete;
                    self.stats.leechers = response.incomplete;
                    self.stats.announce_count += 1;
                }
                Err(e) => {
                    log_warn!("Completion announce failed, continuing: {}", e);
                }
            }
        }

        if outcome.scrape_due {
            let plan = self.build_scrape_plan();
            let result = plan.execute().await;
            self.apply_scrape_result(&result, now);
        }

        if outcome.announce_due {
            let plan = self.build_periodic_announce_plan();
            let result = plan.execute().await;
            self.apply_periodic_announce_result(result);
        }

        if outcome.stop {
            self.apply_post_stop_action().await?;
        }

        Ok(())
    }

    fn tick(&mut self, now: Instant) -> UpdateOutcome {
        let elapsed = now.duration_since(self.last_update);
        self.last_update = now;

        if self.check_stop_conditions(&self.stats) {
            self.stats.current_upload_rate = 0.0;
            self.stats.current_download_rate = 0.0;
            return UpdateOutcome {
                completed: false,
                stop: true,
                scrape_due: false,
                announce_due: false,
            };
        }

        let inputs = self.build_tick_inputs(elapsed);
        let (base_upload_rate, base_download_rate) = self.calc_base_rates(&inputs);
        let (upload_rate, download_rate) =
            self.apply_randomized_rates(base_upload_rate, base_download_rate, inputs.left);
        let (mut upload_rate, mut download_rate, is_idling, idling_reason) =
            Self::apply_idling_rules(&inputs, upload_rate, download_rate);

        // Preserve idling state if stop condition was met with post_stop_action=Idle
        let (is_idling, idling_reason) = if self.stats.stop_condition_met
            && self.config.post_stop_action == PostStopAction::Idle
        {
            upload_rate = 0.0;
            download_rate = 0.0;
            (true, Some("stop_condition_met".to_string()))
        } else {
            (is_idling, idling_reason)
        };

        self.stats.is_idling = is_idling;
        self.stats.idling_reason = idling_reason;

        let completed = Self::apply_rate_and_transfer_updates(
            &mut self.stats,
            upload_rate,
            download_rate,
            inputs.elapsed,
        );

        Self::apply_derived_updates(&mut self.stats, now, &inputs);

        self.compute_tick_outcome(&self.stats, now, &inputs, completed)
    }

    fn build_tick_inputs(&self, elapsed: Duration) -> TickInputs {
        let stats = &self.stats;
        let elapsed_secs = stats.elapsed_time.as_secs();

        TickInputs {
            elapsed,
            elapsed_secs,
            left: stats.left,
            seeders: stats.seeders,
            leechers: stats.leechers,
            announce_count: stats.announce_count,
            torrent_size: self.torrent.total_size,
            start_time: self.start_time,
            config: self.config.clone(),
        }
    }

    fn calc_base_rates(&self, inputs: &TickInputs) -> (f64, f64) {
        let config = &inputs.config;

        let base_upload_rate = if config.progressive_rates {
            self.calculate_progressive_rate(
                config.upload_rate,
                config.target_upload_rate.unwrap_or(config.upload_rate),
                inputs.elapsed_secs,
                config.progressive_duration,
            )
        } else {
            config.upload_rate
        };

        let base_download_rate = if config.progressive_rates {
            self.calculate_progressive_rate(
                config.download_rate,
                config.target_download_rate.unwrap_or(config.download_rate),
                inputs.elapsed_secs,
                config.progressive_duration,
            )
        } else {
            config.download_rate
        };

        (base_upload_rate, base_download_rate)
    }

    fn apply_randomized_rates(
        &self,
        base_upload_rate: f64,
        base_download_rate: f64,
        left: u64,
    ) -> (f64, f64) {
        let upload_rate = self.apply_randomization(base_upload_rate);
        let download_rate =
            if left == 0 { 0.0 } else { self.apply_randomization(base_download_rate) };

        (upload_rate, download_rate)
    }

    fn apply_idling_rules(
        inputs: &TickInputs,
        upload_rate: f64,
        download_rate: f64,
    ) -> (f64, f64, bool, Option<String>) {
        let mut upload = upload_rate;
        let mut download = download_rate;
        let mut is_idling = false;
        let mut idling_reason = None;

        let config = &inputs.config;

        if config.idle_when_no_leechers && inputs.leechers == 0 && inputs.announce_count > 0 {
            log_debug!(
                "Idling: no leechers to upload to (leechers={}, announce_count={})",
                inputs.leechers,
                inputs.announce_count
            );
            upload = 0.0;
            is_idling = true;
            idling_reason = Some("no_leechers".to_string());
        }

        if config.idle_when_no_seeders
            && inputs.seeders == 0
            && inputs.left > 0
            && inputs.announce_count > 0
        {
            log_debug!(
                "Idling: no seeders to download from (seeders={}, left={}, announce_count={})",
                inputs.seeders,
                inputs.left,
                inputs.announce_count
            );
            download = 0.0;
            if !is_idling {
                is_idling = true;
                idling_reason = Some("no_seeders".to_string());
            }
        }

        (upload, download, is_idling, idling_reason)
    }

    fn apply_rate_and_transfer_updates(
        stats: &mut FakerStats,
        upload_rate: f64,
        download_rate: f64,
        elapsed: Duration,
    ) -> bool {
        Self::update_rate_stats(stats, upload_rate, download_rate);

        let upload_delta = (upload_rate * 1024.0 * elapsed.as_secs_f64()) as u64;
        let download_delta = (download_rate * 1024.0 * elapsed.as_secs_f64()) as u64;

        log_trace!(
            "Update: elapsed={:.2}s, upload_rate={:.2} KB/s, download_rate={:.2} KB/s, upload_delta={} bytes",
            elapsed.as_secs_f64(),
            upload_rate,
            download_rate,
            upload_delta
        );

        Self::update_transfer_stats(stats, upload_delta, download_delta)
    }

    fn apply_derived_updates(stats: &mut FakerStats, now: Instant, inputs: &TickInputs) {
        Self::update_derived_stats_with_size(
            stats,
            now,
            inputs.torrent_size,
            &inputs.config,
            inputs.start_time,
        );
    }

    fn compute_tick_outcome(
        &self,
        stats: &FakerStats,
        now: Instant,
        inputs: &TickInputs,
        completed: bool,
    ) -> UpdateOutcome {
        let stop = self.check_stop_conditions(stats);

        let scrape_due = self.scrape_supported
            && now.duration_since(self.last_scrape).as_secs() >= inputs.config.scrape_interval;

        let announce_due = stats.next_announce.is_some_and(|next_announce| now >= next_announce);

        UpdateOutcome { completed, stop, scrape_due, announce_due }
    }

    fn build_periodic_announce_plan(&self) -> AnnouncePlan {
        AnnouncePlan {
            tracker_client: Arc::clone(&self.tracker_client),
            tracker_url: self.torrent.get_tracker_url().to_string(),
            request: self.build_announce_request(TrackerEvent::None),
        }
    }

    fn build_scrape_plan(&self) -> ScrapePlan {
        ScrapePlan {
            tracker_client: Arc::clone(&self.tracker_client),
            tracker_url: self.torrent.get_tracker_url().to_string(),
            info_hash: self.torrent.info_hash,
        }
    }

    fn apply_scrape_result(
        &mut self,
        result: &Result<crate::protocol::ScrapeResponse>,
        now: Instant,
    ) {
        match result {
            Ok(scrape_response) => {
                self.stats.seeders = scrape_response.complete;
                self.stats.leechers = scrape_response.incomplete;
                self.last_scrape = now;
                log_debug!(
                    "Scrape updated peer counts: seeders={}, leechers={}",
                    scrape_response.complete,
                    scrape_response.incomplete
                );
            }
            Err(e) => {
                log_warn!("Scrape failed, disabling periodic scrape: {}", e);
                self.scrape_supported = false;
            }
        }
    }

    fn apply_periodic_announce_result(&mut self, result: Result<AnnounceResponse>) {
        match result {
            Ok(response) => {
                self.announce_interval = Duration::from_secs(response.interval as u64);
                self.stats.seeders = response.complete;
                self.stats.leechers = response.incomplete;
                self.stats.last_announce = Some(Instant::now());
                self.stats.next_announce = Some(Instant::now() + self.announce_interval);
                self.stats.announce_count += 1;

                log_info!(
                    "Periodic announce complete. Seeders: {}, Leechers: {}",
                    response.complete,
                    response.incomplete
                );
            }
            Err(e) => {
                log_warn!("Periodic announce failed, will retry next cycle: {}", e);
                self.stats.next_announce = Some(Instant::now() + Duration::from_secs(30));
            }
        }
    }

    pub const fn announce_count(&self) -> u32 {
        self.stats.announce_count
    }

    /// Update only the stats without announcing to tracker (for live updates)
    pub async fn update_stats_only(&mut self) -> Result<()> {
        let now = Instant::now();
        let outcome = self.tick(now);

        if outcome.completed {
            let plan = AnnouncePlan {
                tracker_client: Arc::clone(&self.tracker_client),
                tracker_url: self.torrent.get_tracker_url().to_string(),
                request: self.build_announce_request(TrackerEvent::Completed),
            };
            match plan.execute().await {
                Ok(response) => {
                    self.stats.seeders = response.complete;
                    self.stats.leechers = response.incomplete;
                    self.stats.announce_count += 1;
                }
                Err(e) => {
                    log_warn!("Completion announce failed, continuing: {}", e);
                }
            }
        }

        if outcome.scrape_due {
            let plan = self.build_scrape_plan();
            let result = plan.execute().await;
            self.apply_scrape_result(&result, now);
        }

        if outcome.stop {
            self.apply_post_stop_action().await?;
        }

        Ok(())
    }

    /// Get current stats
    pub fn get_stats(&self) -> FakerStats {
        self.stats.clone()
    }

    /// Build a stats snapshot from config without cloning runtime stats
    pub fn stats_from_config(config: &FakerConfig) -> FakerStats {
        FakerStats {
            uploaded: config.initial_uploaded,
            downloaded: config.initial_downloaded,
            ratio: if config.initial_downloaded > 0 {
                config.initial_uploaded as f64 / config.initial_downloaded as f64
            } else {
                0.0
            },
            left: 0,
            torrent_completion: config.completion_percent.clamp(0.0, 100.0),
            seeders: 0,
            leechers: 0,
            state: FakerState::Stopped,
            is_idling: false,
            idling_reason: None,
            session_uploaded: 0,
            session_downloaded: 0,
            session_ratio: 0.0,
            elapsed_time: Duration::from_secs(0),
            current_upload_rate: 0.0,
            current_download_rate: 0.0,
            average_upload_rate: 0.0,
            average_download_rate: 0.0,
            upload_progress: 0.0,
            download_progress: 0.0,
            ratio_progress: 0.0,
            seed_time_progress: 0.0,
            effective_stop_at_ratio: config.stop_at_ratio,
            eta_ratio: None,
            eta_uploaded: None,
            eta_seed_time: None,
            eta_download_completion: None,
            upload_rate_history: Vec::new(),
            download_rate_history: Vec::new(),
            ratio_history: Vec::new(),
            history_timestamps: Vec::new(),
            last_announce: None,
            next_announce: None,
            announce_count: 0,
            stop_condition_met: false,
            post_stop_action: config.post_stop_action,
        }
    }

    /// Non-async stats snapshot (for synchronous exit-save contexts)
    pub fn stats_snapshot(&self) -> FakerStats {
        self.stats.clone()
    }

    pub fn peer_id(&self) -> &str {
        &self.peer_id
    }

    pub fn info_hash(&self) -> [u8; 20] {
        self.torrent.info_hash
    }

    pub const fn port(&self) -> u16 {
        self.config.port
    }

    /// Get torrent info
    pub const fn get_torrent(&self) -> &Arc<TorrentInfo> {
        &self.torrent
    }

    /// Update the faker's configuration in-place without recreating the entire struct.
    ///
    /// This avoids re-allocating the `reqwest::Client` and other internal state.
    /// Only recreates the `TrackerClient` if the `client_type` changed (which changes peer ID / User-Agent).
    pub fn update_config(
        &mut self,
        config: FakerConfig,
        http_client: Option<reqwest::Client>,
    ) -> Result<()> {
        let mut config = config;
        let client_type_changed = config.client_type != self.config.client_type
            || config.client_version != self.config.client_version;

        if client_type_changed {
            let client_config =
                ClientConfig::get(config.client_type, config.client_version.clone());
            self.peer_id = client_config.generate_peer_id();
            self.key = ClientConfig::generate_key();
            self.tracker_client = Arc::new(
                TrackerClient::new(client_config, http_client)
                    .map_err(|e| FakerError::ConfigError(e.to_string()))?,
            );
        }

        // Recompute left/torrent_completion from the new completion_percent.
        // This ensures that changing completion_percent in the UI takes effect
        // without having to recreate the faker.
        let completion = config.completion_percent.clamp(0.0, 100.0) / 100.0;
        let torrent_downloaded = (self.torrent.total_size as f64 * completion) as u64;
        let new_left = self.torrent.total_size.saturating_sub(torrent_downloaded);
        let new_torrent_completion = if self.torrent.total_size > 0 {
            ((self.torrent.total_size - new_left) as f64 / self.torrent.total_size as f64) * 100.0
        } else {
            100.0
        };

        self.stats.left = new_left;
        self.stats.torrent_completion = new_torrent_completion;

        Self::resolve_stop_ratio(&mut config);
        self.stats.effective_stop_at_ratio = config.stop_at_ratio;

        self.config = config;
        Ok(())
    }

    fn build_announce_request(&self, event: TrackerEvent) -> AnnounceRequest {
        log_debug!(
            "Preparing announce: event={:?}, uploaded={}, downloaded={}, left={}",
            event,
            self.stats.uploaded,
            self.stats.downloaded,
            self.stats.left
        );

        AnnounceRequest {
            info_hash: self.torrent.info_hash,
            peer_id: self.peer_id.clone(),
            port: self.config.port,
            uploaded: self.stats.uploaded,
            downloaded: self.stats.downloaded,
            left: self.stats.left,
            compact: true,
            no_peer_id: false,
            event,
            ip: None,
            numwant: Some(self.config.num_want),
            key: Some(self.key.clone()),
            tracker_id: self.tracker_id.clone(),
        }
    }

    /// Scrape the tracker for stats
    pub async fn scrape(&self) -> Result<crate::protocol::ScrapeResponse> {
        let plan = self.build_scrape_plan();
        let response = plan.execute().await?;
        log_info!(
            "Scrape complete. Seeders: {}, Leechers: {}, Downloaded: {}",
            response.complete,
            response.incomplete,
            response.downloaded
        );
        Ok(response)
    }

    /// Pause the faker
    pub fn pause(&mut self) -> Result<()> {
        log_info!("Pausing ratio faker");
        self.stats.state = FakerState::Paused;
        self.stats.is_idling = false;
        self.stats.idling_reason = None;
        Ok(())
    }

    /// Resume the faker
    pub fn resume(&mut self) -> Result<()> {
        log_info!("Resuming ratio faker");
        let now = Instant::now();
        self.stats.state = FakerState::Running;
        self.rebase_timers_from_elapsed(now);
        if self.stats.next_announce.is_none() {
            self.stats.next_announce = Some(now);
        }
        Ok(())
    }

    /// Apply randomization to a rate if enabled
    fn apply_randomization(&self, base_rate: f64) -> f64 {
        if self.config.randomize_rates {
            let mut rng = rand::rng();
            let range = self.config.random_range_percent / 100.0;
            let variation = 1.0 + rng.random::<f64>().mul_add(range * 2.0, -range);
            base_rate * variation
        } else {
            base_rate
        }
    }

    /// Update rate statistics and history
    fn update_rate_stats(stats: &mut FakerStats, upload_rate: f64, download_rate: f64) {
        stats.current_upload_rate = upload_rate;
        stats.current_download_rate = download_rate;

        // Record timestamp for this data point (Unix millis)
        let timestamp = Self::current_timestamp_millis();
        Self::add_to_history_u64(&mut stats.history_timestamps, timestamp, 60);

        Self::add_to_history(&mut stats.upload_rate_history, upload_rate, 60);
        Self::add_to_history(&mut stats.download_rate_history, download_rate, 60);
    }

    /// Update transfer stats (uploaded, downloaded, left). Returns true if just completed.
    #[allow(clippy::unused_self)]
    fn update_transfer_stats(
        stats: &mut FakerStats,
        upload_delta: u64,
        download_delta: u64,
    ) -> bool {
        stats.uploaded += upload_delta;
        stats.session_uploaded += upload_delta;

        if stats.left > 0 {
            let actual_download = download_delta.min(stats.left);
            stats.downloaded += actual_download;
            stats.session_downloaded += actual_download;
            stats.left = stats.left.saturating_sub(actual_download);

            stats.left == 0
        } else {
            false
        }
    }

    /// Update derived statistics (ratio, elapsed time, average rates, progress)
    fn update_derived_stats_with_size(
        stats: &mut FakerStats,
        now: Instant,
        torrent_size: u64,
        config: &FakerConfig,
        start_time: Instant,
    ) {
        // Cumulative ratio (for display in Total Stats)
        let current_ratio =
            if torrent_size > 0 { stats.uploaded as f64 / torrent_size as f64 } else { 0.0 };
        stats.ratio = current_ratio;
        Self::add_to_history(&mut stats.ratio_history, current_ratio, 60);

        // Session ratio = session_uploaded / torrent_size
        stats.session_ratio = if torrent_size > 0 {
            stats.session_uploaded as f64 / torrent_size as f64
        } else {
            0.0
        };

        stats.elapsed_time = now.duration_since(start_time);

        // Torrent completion percentage
        stats.torrent_completion = if torrent_size > 0 {
            ((torrent_size - stats.left) as f64 / torrent_size as f64) * 100.0
        } else {
            100.0
        };

        let elapsed_secs = stats.elapsed_time.as_secs_f64();
        if elapsed_secs > 0.0 {
            stats.average_upload_rate = (stats.session_uploaded as f64 / 1024.0) / elapsed_secs;
            stats.average_download_rate = (stats.session_downloaded as f64 / 1024.0) / elapsed_secs;
        }

        Self::update_progress_and_eta_with_size(stats, config, torrent_size);
    }

    /// Add a value to a history vec, keeping only the last `max_len` items
    fn add_to_history(history: &mut Vec<f64>, value: f64, max_len: usize) {
        history.push(value);
        if history.len() > max_len {
            history.remove(0);
        }
    }

    /// Add a u64 value to a history vec, keeping only the last `max_len` items
    fn add_to_history_u64(history: &mut Vec<u64>, value: u64, max_len: usize) {
        history.push(value);
        if history.len() > max_len {
            history.remove(0);
        }
    }

    /// Get current timestamp in milliseconds (cross-platform)
    fn current_timestamp_millis() -> u64 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
        }
        #[cfg(target_arch = "wasm32")]
        {
            js_sys::Date::now() as u64
        }
    }

    fn check_stop_conditions(&self, stats: &FakerStats) -> bool {
        // Don't re-trigger if already met
        if stats.stop_condition_met {
            return false;
        }

        // Check ratio target (cumulative across all sessions)
        if let Some(target_ratio) = self.config.stop_at_ratio {
            if stats.ratio >= target_ratio - 0.001 {
                log_info!(
                    "Target ratio reached: {:.3} >= {:.3} (cumulative)",
                    stats.ratio,
                    target_ratio
                );
                return true;
            }
        }

        // Check uploaded target (cumulative across all sessions)
        if let Some(target_uploaded) = self.config.stop_at_uploaded {
            if stats.uploaded >= target_uploaded {
                log_info!(
                    "Target uploaded reached: {} >= {} bytes (cumulative)",
                    stats.uploaded,
                    target_uploaded
                );
                return true;
            }
        }

        // Check downloaded target (cumulative across all sessions)
        if let Some(target_downloaded) = self.config.stop_at_downloaded {
            if stats.downloaded >= target_downloaded {
                log_info!(
                    "Target downloaded reached: {} >= {} bytes (cumulative)",
                    stats.downloaded,
                    target_downloaded
                );
                return true;
            }
        }

        // Check seed time target
        if let Some(target_seed_time) = self.config.stop_at_seed_time {
            if stats.elapsed_time.as_secs() >= target_seed_time {
                log_info!(
                    "Target seed time reached: {}s >= {}s",
                    stats.elapsed_time.as_secs(),
                    target_seed_time
                );
                return true;
            }
        }

        false
    }

    /// Calculate progressive rate (linear interpolation)
    #[allow(clippy::unused_self)]
    fn calculate_progressive_rate(
        &self,
        start_rate: f64,
        target_rate: f64,
        elapsed_secs: u64,
        duration_secs: u64,
    ) -> f64 {
        if elapsed_secs >= duration_secs {
            return target_rate;
        }

        let progress = elapsed_secs as f64 / duration_secs as f64;
        (target_rate - start_rate).mul_add(progress, start_rate)
    }

    /// Update progress percentages and ETAs
    fn update_progress_and_eta_with_size(
        stats: &mut FakerStats,
        config: &FakerConfig,
        torrent_size: u64,
    ) {
        // Upload progress (based on session uploaded)
        if let Some(target) = config.stop_at_uploaded {
            stats.upload_progress =
                ((stats.session_uploaded as f64 / target as f64) * 100.0).min(100.0);

            // Calculate ETA
            if stats.average_upload_rate > 0.0 {
                let remaining = target.saturating_sub(stats.session_uploaded);
                let eta_secs = (remaining as f64 / 1024.0) / stats.average_upload_rate;
                stats.eta_uploaded = Some(Duration::from_secs_f64(eta_secs));
            }
        } else {
            stats.upload_progress = 0.0;
            stats.eta_uploaded = None;
        }

        // Download progress (based on session downloaded)
        if let Some(target) = config.stop_at_downloaded {
            stats.download_progress =
                ((stats.session_downloaded as f64 / target as f64) * 100.0).min(100.0);
        } else {
            stats.download_progress = 0.0;
        }

        // Ratio progress (use cumulative ratio for progress tracking)
        if let Some(target_ratio) = config.stop_at_ratio {
            stats.ratio_progress = ((stats.ratio / target_ratio) * 100.0).min(100.0);

            // Calculate ETA for ratio (based on cumulative stats)
            if stats.average_upload_rate > 0.0 && torrent_size > 0 {
                let target_total_uploaded = (target_ratio * torrent_size as f64) as u64;
                let remaining = target_total_uploaded.saturating_sub(stats.uploaded);
                let eta_secs = (remaining as f64 / 1024.0) / stats.average_upload_rate;
                stats.eta_ratio = Some(Duration::from_secs_f64(eta_secs));
            }
        } else {
            stats.ratio_progress = 0.0;
            stats.eta_ratio = None;
        }

        // Seed time progress
        if let Some(target_time) = config.stop_at_seed_time {
            let elapsed = stats.elapsed_time.as_secs();
            stats.seed_time_progress = ((elapsed as f64 / target_time as f64) * 100.0).min(100.0);

            let remaining = target_time.saturating_sub(elapsed);
            stats.eta_seed_time = Some(Duration::from_secs(remaining));
        } else {
            stats.seed_time_progress = 0.0;
            stats.eta_seed_time = None;
        }

        // Download completion ETA (time until torrent fully downloaded)
        if stats.left > 0 && stats.average_download_rate > 0.0 {
            let eta_secs = (stats.left as f64 / 1024.0) / stats.average_download_rate;
            stats.eta_download_completion = Some(Duration::from_secs_f64(eta_secs));
        } else {
            stats.eta_download_completion = None;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct RatioFakerHandle {
    inner: Arc<Mutex<RatioFaker>>,
    stats_tx: watch::Sender<FakerStats>,
    stats_rx: watch::Receiver<FakerStats>,
}

#[cfg(not(target_arch = "wasm32"))]
impl RatioFakerHandle {
    pub fn new(faker: RatioFaker) -> Self {
        let stats = faker.stats_snapshot();
        let (stats_tx, stats_rx) = watch::channel(stats);
        Self { inner: Arc::new(Mutex::new(faker)), stats_tx, stats_rx }
    }

    pub fn stats_snapshot(&self) -> FakerStats {
        self.stats_rx.borrow().clone()
    }

    pub async fn start(&self) -> Result<()> {
        let plan = {
            let mut guard = self.inner.lock().await;
            guard.begin_start()
        };

        if let Some(plan) = plan {
            let result = plan.execute().await;
            let mut guard = self.inner.lock().await;
            guard.apply_start_result(result);
            let _ = self.stats_tx.send(guard.stats_snapshot());
        }
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let plan = {
            let mut guard = self.inner.lock().await;
            guard.begin_stop()
        };

        if let Some(plan) = plan {
            let result = plan.execute().await;
            let mut guard = self.inner.lock().await;
            guard.apply_stop_result(result);
            let _ = self.stats_tx.send(guard.stats_snapshot());
        }
        Ok(())
    }

    pub async fn pause(&self) -> Result<()> {
        let mut guard = self.inner.lock().await;
        let result = guard.pause();
        let _ = self.stats_tx.send(guard.stats_snapshot());
        result
    }

    pub async fn resume(&self) -> Result<()> {
        let mut guard = self.inner.lock().await;
        let result = guard.resume();
        let _ = self.stats_tx.send(guard.stats_snapshot());
        result
    }

    pub async fn restore_running(&self) -> Result<()> {
        let plan = {
            let mut guard = self.inner.lock().await;
            guard.begin_restore_running()
        };

        let result = plan.execute().await;
        let mut guard = self.inner.lock().await;
        guard.apply_start_result(result);
        let _ = self.stats_tx.send(guard.stats_snapshot());
        Ok(())
    }

    pub async fn restore_snapshot(&self, stats: FakerStats) {
        let mut guard = self.inner.lock().await;
        guard.restore_runtime(stats);
        let _ = self.stats_tx.send(guard.stats_snapshot());
    }

    async fn apply_post_stop_action(&self) -> Result<()> {
        let post_stop_action = {
            let guard = self.inner.lock().await;
            guard.config.post_stop_action
        };
        match post_stop_action {
            PostStopAction::Idle => {
                log_info!("Stop condition met, idling (post_stop_action=idle)");
                let mut guard = self.inner.lock().await;
                guard.stats.stop_condition_met = true;
                guard.stats.is_idling = true;
                guard.stats.idling_reason = Some("stop_condition_met".to_string());
                guard.stats.current_upload_rate = 0.0;
                guard.stats.current_download_rate = 0.0;
            }
            PostStopAction::StopSeeding | PostStopAction::DeleteInstance => {
                log_info!("Stop condition met, stopping faker");
                let plan = {
                    let mut guard = self.inner.lock().await;
                    guard.stats.stop_condition_met = true;
                    guard.begin_stop()
                };
                if let Some(plan) = plan {
                    let result = plan.execute().await;
                    let mut guard = self.inner.lock().await;
                    guard.apply_stop_result(result);
                }
            }
        }
        Ok(())
    }

    pub async fn update(&self) -> Result<()> {
        let now = Instant::now();
        let outcome = {
            let mut guard = self.inner.lock().await;
            guard.tick(now)
        };

        if outcome.completed {
            let plan = {
                let guard = self.inner.lock().await;
                AnnouncePlan {
                    tracker_client: Arc::clone(&guard.tracker_client),
                    tracker_url: guard.torrent.get_tracker_url().to_string(),
                    request: guard.build_announce_request(TrackerEvent::Completed),
                }
            };
            if let Ok(response) = plan.execute().await {
                let mut guard = self.inner.lock().await;
                guard.stats.seeders = response.complete;
                guard.stats.leechers = response.incomplete;
                guard.stats.announce_count += 1;
            }
        }

        if outcome.scrape_due {
            let plan = {
                let guard = self.inner.lock().await;
                guard.build_scrape_plan()
            };
            let result = plan.execute().await;
            let mut guard = self.inner.lock().await;
            guard.apply_scrape_result(&result, now);
        }

        if outcome.announce_due {
            let plan = {
                let guard = self.inner.lock().await;
                guard.build_periodic_announce_plan()
            };
            let result = plan.execute().await;
            let mut guard = self.inner.lock().await;
            guard.apply_periodic_announce_result(result);
        }

        if outcome.stop {
            self.apply_post_stop_action().await?;
        }

        let guard = self.inner.lock().await;
        let _ = self.stats_tx.send(guard.stats_snapshot());
        Ok(())
    }

    pub async fn update_stats_only(&self) -> Result<()> {
        let now = Instant::now();
        let outcome = {
            let mut guard = self.inner.lock().await;
            guard.tick(now)
        };

        if outcome.completed {
            let plan = {
                let guard = self.inner.lock().await;
                AnnouncePlan {
                    tracker_client: Arc::clone(&guard.tracker_client),
                    tracker_url: guard.torrent.get_tracker_url().to_string(),
                    request: guard.build_announce_request(TrackerEvent::Completed),
                }
            };
            if let Ok(response) = plan.execute().await {
                let mut guard = self.inner.lock().await;
                guard.stats.seeders = response.complete;
                guard.stats.leechers = response.incomplete;
                guard.stats.announce_count += 1;
            }
        }

        if outcome.scrape_due {
            let plan = {
                let guard = self.inner.lock().await;
                guard.build_scrape_plan()
            };
            let result = plan.execute().await;
            let mut guard = self.inner.lock().await;
            guard.apply_scrape_result(&result, now);
        }

        if outcome.stop {
            self.apply_post_stop_action().await?;
        }

        let guard = self.inner.lock().await;
        let _ = self.stats_tx.send(guard.stats_snapshot());
        Ok(())
    }

    pub async fn scrape(&self) -> Result<crate::protocol::ScrapeResponse> {
        let plan = {
            let guard = self.inner.lock().await;
            guard.build_scrape_plan()
        };
        let result = plan.execute().await;
        let now = Instant::now();
        let mut guard = self.inner.lock().await;
        guard.apply_scrape_result(&result, now);
        let _ = self.stats_tx.send(guard.stats_snapshot());
        result
    }

    pub async fn update_config(
        &self,
        config: FakerConfig,
        http_client: Option<reqwest::Client>,
    ) -> Result<()> {
        let mut guard = self.inner.lock().await;
        let result = guard.update_config(config, http_client);
        let _ = self.stats_tx.send(guard.stats_snapshot());
        result
    }

    pub async fn peer_id(&self) -> String {
        let guard = self.inner.lock().await;
        guard.peer_id().to_string()
    }

    pub async fn peer_id_bytes(
        &self,
    ) -> std::result::Result<[u8; 20], crate::protocol::PeerProtocolError> {
        let peer_id = self.peer_id().await;
        peer_id_to_array(&peer_id)
    }

    pub async fn info_hash(&self) -> [u8; 20] {
        let guard = self.inner.lock().await;
        guard.info_hash()
    }

    pub async fn effective_port(&self) -> u16 {
        let guard = self.inner.lock().await;
        guard.port()
    }

    pub async fn set_runtime_port(&self, port: u16) {
        let mut guard = self.inner.lock().await;
        guard.config.port = port;
        let _ = self.stats_tx.send(guard.stats_snapshot());
    }

    pub async fn is_peer_connectable(&self) -> bool {
        let guard = self.inner.lock().await;
        handle_is_connectable(guard.stats.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_faker_config_default() {
        let config = FakerConfig::default();
        assert_eq!(config.upload_rate, 50.0);
        assert_eq!(config.download_rate, 100.0);
        assert!(!config.vpn_port_sync);
    }

    #[test]
    fn test_preset_settings_to_faker_config_defaults() {
        let preset = PresetSettings::default();
        let config: FakerConfig = preset.into();

        assert_eq!(config.upload_rate, 50.0);
        assert_eq!(config.download_rate, 100.0);
        assert_eq!(config.port, 6881);
        assert!(!config.vpn_port_sync);
        assert_eq!(config.client_type, ClientType::QBittorrent);
        assert_eq!(config.completion_percent, 100.0);
        assert!(config.randomize_rates);
        assert_eq!(config.random_range_percent, 20.0);
        assert!(!config.randomize_ratio);
        assert_eq!(config.random_ratio_range_percent, 10.0);
        assert!(config.stop_at_ratio.is_none());
        assert!(config.stop_at_uploaded.is_none());
        assert!(!config.progressive_rates);
    }

    #[test]
    fn test_preset_settings_to_faker_config_with_values() {
        let preset = PresetSettings {
            upload_rate: Some(100.0),
            download_rate: Some(200.0),
            port: Some(51413),
            vpn_port_sync: Some(true),
            selected_client: Some(ClientType::Transmission),
            completion_percent: Some(50.0),
            randomize_rates: Some(false),
            stop_at_ratio_enabled: Some(true),
            stop_at_ratio: Some(2.5),
            stop_at_uploaded_enabled: Some(true),
            stop_at_uploaded_gb: Some(10.0),
            stop_at_seed_time_enabled: Some(true),
            stop_at_seed_time_hours: Some(24.0),
            progressive_rates_enabled: Some(true),
            target_upload_rate: Some(500.0),
            progressive_duration_hours: Some(2.0),
            ..Default::default()
        };
        let config: FakerConfig = preset.into();

        assert_eq!(config.upload_rate, 100.0);
        assert_eq!(config.download_rate, 200.0);
        assert_eq!(config.port, 51413);
        assert!(config.vpn_port_sync);
        assert_eq!(config.client_type, ClientType::Transmission);
        assert_eq!(config.completion_percent, 50.0);
        assert!(!config.randomize_rates);
        assert_eq!(config.stop_at_ratio, Some(2.5));
        // 10 GB in bytes
        assert_eq!(config.stop_at_uploaded, Some(10 * 1024 * 1024 * 1024));
        // 24 hours in seconds
        assert_eq!(config.stop_at_seed_time, Some(24 * 3600));
        assert!(config.progressive_rates);
        assert_eq!(config.target_upload_rate, Some(500.0));
        // 2 hours in seconds
        assert_eq!(config.progressive_duration, 2 * 3600);
    }

    #[test]
    fn test_preset_settings_disabled_stop_conditions() {
        let preset = PresetSettings {
            stop_at_ratio_enabled: Some(false),
            stop_at_ratio: Some(2.0),
            stop_at_uploaded_enabled: Some(false),
            stop_at_uploaded_gb: Some(50.0),
            ..Default::default()
        };
        let config: FakerConfig = preset.into();

        // Even though values are set, they should be None because enabled is false
        assert!(config.stop_at_ratio.is_none());
        assert!(config.stop_at_uploaded.is_none());
    }

    #[test]
    fn test_faker_config_deserializes_missing_vpn_port_sync_as_false() {
        let json = r#"{
            "upload_rate": 50.0,
            "download_rate": 100.0,
            "port": 6881,
            "client_type": "qbittorrent",
            "client_version": null,
            "initial_uploaded": 0,
            "initial_downloaded": 0,
            "completion_percent": 100.0,
            "num_want": 50,
            "randomize_rates": true,
            "random_range_percent": 20.0,
            "randomize_ratio": false,
            "random_ratio_range_percent": 10.0,
            "stop_at_ratio": null,
            "effective_stop_at_ratio": null,
            "stop_at_uploaded": null,
            "stop_at_downloaded": null,
            "stop_at_seed_time": null,
            "idle_when_no_leechers": false,
            "idle_when_no_seeders": false,
            "scrape_interval": 60,
            "progressive_rates": false,
            "target_upload_rate": null,
            "target_download_rate": null,
            "progressive_duration": 3600
        }"#;

        let parsed = serde_json::from_str::<FakerConfig>(json);
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap_or_default();
        assert!(!parsed.vpn_port_sync);
    }

    #[test]
    fn update_config_uses_precomputed_effective_stop_ratio() {
        let torrent = Arc::new(TorrentInfo {
            info_hash: [7u8; 20],
            announce: "https://tracker.test/announce".to_string(),
            announce_list: None,
            name: "sample".to_string(),
            total_size: 1024,
            piece_length: 256,
            num_pieces: 4,
            creation_date: None,
            comment: None,
            created_by: None,
            is_single_file: true,
            file_count: 1,
            files: Vec::new(),
        });

        let faker = RatioFaker::new(
            torrent,
            FakerConfig {
                stop_at_ratio: Some(2.0),
                randomize_ratio: true,
                effective_stop_at_ratio: Some(1.8689),
                ..FakerConfig::default()
            },
            None,
        );
        assert!(faker.is_ok());
        let mut faker = faker.unwrap_or_else(|_| panic!("failed to create faker"));

        let updated = faker.update_config(
            FakerConfig {
                stop_at_ratio: Some(2.0),
                randomize_ratio: true,
                effective_stop_at_ratio: Some(1.8689),
                ..FakerConfig::default()
            },
            None,
        );
        assert!(updated.is_ok());

        assert_eq!(faker.config.stop_at_ratio, Some(1.8689));
        assert_eq!(faker.stats.effective_stop_at_ratio, Some(1.8689));
    }

    #[test]
    fn start_resets_session_stats_but_keeps_cumulative_ratio_progress() {
        let torrent = Arc::new(TorrentInfo {
            info_hash: [9u8; 20],
            announce: "https://tracker.test/announce".to_string(),
            announce_list: None,
            name: "sample".to_string(),
            total_size: 1024,
            piece_length: 256,
            num_pieces: 4,
            creation_date: None,
            comment: None,
            created_by: None,
            is_single_file: true,
            file_count: 1,
            files: Vec::new(),
        });

        let faker = RatioFaker::new(
            torrent,
            FakerConfig {
                initial_uploaded: 10 * 1024,
                initial_downloaded: 5 * 1024,
                stop_at_ratio: Some(20.0),
                ..FakerConfig::default()
            },
            None,
        );
        assert!(faker.is_ok());
        let mut faker = faker.unwrap_or_else(|_| panic!("failed to create faker"));

        faker.stats.session_uploaded = 1234;
        faker.stats.session_downloaded = 567;
        faker.stats.session_ratio = 1.5;
        faker.stats.ratio_progress = 42.0;
        faker.stats.announce_count = 9;

        let plan = faker.begin_start();
        assert!(plan.is_some());
        assert_eq!(faker.stats.session_uploaded, 0);
        assert_eq!(faker.stats.session_downloaded, 0);
        assert_eq!(faker.stats.session_ratio, 0.0);
        assert_eq!(faker.stats.ratio_progress, 0.0);
        assert_eq!(faker.stats.announce_count, 0);
        assert!(faker.stats.ratio > 0.0);
        assert!(!faker.check_stop_conditions(&faker.stats));
    }

    #[test]
    fn tick_stops_without_extra_transfer_when_condition_already_met() {
        let torrent = Arc::new(TorrentInfo {
            info_hash: [11u8; 20],
            announce: "https://tracker.test/announce".to_string(),
            announce_list: None,
            name: "sample".to_string(),
            total_size: 1024,
            piece_length: 256,
            num_pieces: 4,
            creation_date: None,
            comment: None,
            created_by: None,
            is_single_file: true,
            file_count: 1,
            files: Vec::new(),
        });

        let faker = RatioFaker::new(
            torrent,
            FakerConfig {
                stop_at_ratio: Some(1.0),
                initial_uploaded: 2048,
                completion_percent: 100.0,
                ..FakerConfig::default()
            },
            None,
        );
        assert!(faker.is_ok());
        let mut faker = faker.unwrap_or_else(|_| panic!("failed to create faker"));

        faker.stats.state = FakerState::Running;
        faker.stats.ratio = 2.0;
        faker.last_update =
            faker.last_update.checked_sub(Duration::from_secs(5)).unwrap_or_else(Instant::now);

        let uploaded_before = faker.stats.uploaded;
        let downloaded_before = faker.stats.downloaded;

        let outcome = faker.tick(Instant::now());

        assert!(outcome.stop);
        assert_eq!(faker.stats.uploaded, uploaded_before);
        assert_eq!(faker.stats.downloaded, downloaded_before);
        assert_eq!(faker.stats.current_upload_rate, 0.0);
        assert_eq!(faker.stats.current_download_rate, 0.0);
    }
}
