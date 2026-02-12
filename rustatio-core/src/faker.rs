use crate::protocol::{AnnounceRequest, AnnounceResponse, TrackerClient, TrackerError, TrackerEvent};
use crate::torrent::{ClientConfig, ClientType, TorrentInfo};
use crate::{log_debug, log_info, log_trace, log_warn};
use instant::Instant;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::RwLock;

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

#[cfg(target_arch = "wasm32")]
use js_sys;

// Macros for platform-specific lock access
#[cfg(not(target_arch = "wasm32"))]
macro_rules! read_lock {
    ($lock:expr) => {
        $lock.read().await
    };
}

#[cfg(target_arch = "wasm32")]
macro_rules! read_lock {
    ($lock:expr) => {
        $lock.borrow()
    };
}

#[cfg(not(target_arch = "wasm32"))]
macro_rules! write_lock {
    ($lock:expr) => {
        $lock.write().await
    };
}

#[cfg(target_arch = "wasm32")]
macro_rules! write_lock {
    ($lock:expr) => {
        $lock.borrow_mut()
    };
}

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

    // Stop conditions
    /// Stop when ratio reaches this value (optional)
    pub stop_at_ratio: Option<f64>,

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
}

/// UI-friendly preset settings format (matches frontend)
/// Uses human-readable units (GB, hours) and enabled flags for optional fields
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PresetSettings {
    pub upload_rate: Option<f64>,
    pub download_rate: Option<f64>,
    pub port: Option<u16>,
    pub selected_client: Option<ClientType>,
    pub selected_client_version: Option<String>,
    pub completion_percent: Option<f64>,
    pub randomize_rates: Option<bool>,
    pub random_range_percent: Option<f64>,
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
    // Progressive rates
    pub progressive_rates_enabled: Option<bool>,
    pub target_upload_rate: Option<f64>,
    pub target_download_rate: Option<f64>,
    pub progressive_duration_hours: Option<f64>,
}

impl From<PresetSettings> for FakerConfig {
    fn from(p: PresetSettings) -> Self {
        let stop_at_ratio = if p.stop_at_ratio_enabled.unwrap_or(false) {
            p.stop_at_ratio
        } else {
            None
        };

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

        FakerConfig {
            upload_rate: p.upload_rate.unwrap_or(50.0),
            download_rate: p.download_rate.unwrap_or(100.0),
            port: p.port.unwrap_or(6881),
            client_type: p.selected_client.unwrap_or(ClientType::QBittorrent),
            client_version: p.selected_client_version,
            initial_uploaded: 0,
            initial_downloaded: 0,
            completion_percent: p.completion_percent.unwrap_or(100.0),
            num_want: 50,
            randomize_rates: p.randomize_rates.unwrap_or(true),
            random_range_percent: p.random_range_percent.unwrap_or(20.0),
            stop_at_ratio,
            stop_at_uploaded,
            stop_at_downloaded,
            stop_at_seed_time,
            idle_when_no_leechers: p.idle_when_no_leechers.unwrap_or(false),
            idle_when_no_seeders: p.idle_when_no_seeders.unwrap_or(false),
            scrape_interval: 60,
            progressive_rates: p.progressive_rates_enabled.unwrap_or(false),
            target_upload_rate: p.target_upload_rate,
            target_download_rate: p.target_download_rate,
            progressive_duration: (p.progressive_duration_hours.unwrap_or(1.0) * 3600.0) as u64,
        }
    }
}

fn default_randomize_rates() -> bool {
    true
}

fn default_progressive_duration() -> u64 {
    3600 // 1 hour
}

fn default_random_range() -> f64 {
    20.0
}

fn default_scrape_interval() -> u64 {
    60 // 60 seconds
}

impl Default for FakerConfig {
    fn default() -> Self {
        FakerConfig {
            upload_rate: 50.0,    // 50 KB/s
            download_rate: 100.0, // 100 KB/s
            port: 6881,
            client_type: ClientType::QBittorrent,
            client_version: None,
            initial_uploaded: 0,
            initial_downloaded: 0,
            completion_percent: 0.0,
            num_want: 50,
            randomize_rates: true,
            random_range_percent: 20.0,
            stop_at_ratio: None,
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    // === INTERNAL ===
    #[serde(skip)]
    pub last_announce: Option<Instant>,
    #[serde(skip)]
    pub next_announce: Option<Instant>,
    pub announce_count: u32,
}

#[cfg(not(target_arch = "wasm32"))]
pub struct RatioFaker {
    torrent: TorrentInfo,
    config: FakerConfig,
    tracker_client: TrackerClient,

    // Runtime state
    state: Arc<RwLock<FakerState>>,
    stats: Arc<RwLock<FakerStats>>,

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

#[cfg(target_arch = "wasm32")]
pub struct RatioFaker {
    torrent: TorrentInfo,
    config: FakerConfig,
    tracker_client: TrackerClient,

    // Runtime state (RefCell for single-threaded WASM)
    state: RefCell<FakerState>,
    stats: RefCell<FakerStats>,

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

impl RatioFaker {
    pub fn new(torrent: TorrentInfo, config: FakerConfig) -> Result<Self> {
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
        let client_config = ClientConfig::get(config.client_type.clone(), config.client_version.clone());

        // Generate session identifiers
        let peer_id = client_config.generate_peer_id();
        let key = ClientConfig::generate_key();

        log_trace!("Generated peer_id: {}, key: {}", peer_id, key);

        // Create tracker client
        let tracker_client =
            TrackerClient::new(client_config.clone()).map_err(|e| FakerError::ConfigError(e.to_string()))?;

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
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(RatioFaker {
                torrent,
                config,
                tracker_client,
                state: Arc::new(RwLock::new(FakerState::Stopped)),
                stats: Arc::new(RwLock::new(stats)),
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

        #[cfg(target_arch = "wasm32")]
        {
            Ok(RatioFaker {
                torrent,
                config,
                tracker_client,
                state: RefCell::new(FakerState::Stopped),
                stats: RefCell::new(stats),
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
    }

    /// Start the ratio faking session
    pub async fn start(&mut self) -> Result<()> {
        log_info!("Starting ratio faker for torrent: {}", self.torrent.name);

        // Set transitional Starting state before first announce
        *write_lock!(self.state) = FakerState::Starting;
        write_lock!(self.stats).state = FakerState::Starting;
        self.start_time = Instant::now();
        self.last_update = Instant::now();

        // Send started event — best-effort, don't fail if tracker is unreachable
        match self.announce(TrackerEvent::Started).await {
            Ok(response) => {
                self.announce_interval = Duration::from_secs(response.interval as u64);
                self.tracker_id = response.tracker_id;

                let mut stats = write_lock!(self.stats);
                stats.seeders = response.complete;
                stats.leechers = response.incomplete;
                stats.last_announce = Some(Instant::now());
                stats.next_announce = Some(Instant::now() + self.announce_interval);
                stats.announce_count += 1;

                log_info!(
                    "Started successfully. Seeders: {}, Leechers: {}, Interval: {}s",
                    response.complete,
                    response.incomplete,
                    response.interval
                );
            }
            Err(e) => {
                log_warn!("Initial announce failed, will retry on next cycle: {}", e);
                // Schedule a retry announce soon (30s) instead of waiting the full default interval
                let mut stats = write_lock!(self.stats);
                stats.next_announce = Some(Instant::now() + Duration::from_secs(30));
            }
        }

        // ALWAYS transition to Running regardless of announce result
        *write_lock!(self.state) = FakerState::Running;
        write_lock!(self.stats).state = FakerState::Running;

        Ok(())
    }

    /// Stop the ratio faking session
    pub async fn stop(&mut self) -> Result<()> {
        // Guard: no-op if already stopped (prevents redundant stop announces
        // when the scheduler auto-stops via stop conditions and the frontend
        // subsequently calls stop again after observing the Stopped state)
        if matches!(*read_lock!(self.state), FakerState::Stopped) {
            log_debug!("Already stopped, skipping stop");
            return Ok(());
        }

        log_info!("Stopping ratio faker");

        // Set transitional Stopping state before tracker announce
        *write_lock!(self.state) = FakerState::Stopping;
        write_lock!(self.stats).state = FakerState::Stopping;

        // Send stopped event — best-effort, tracker will time out the peer anyway
        match self.announce(TrackerEvent::Stopped).await {
            Ok(_) => {
                write_lock!(self.stats).announce_count += 1;
                log_info!("Stop announce sent successfully");
            }
            Err(e) => {
                log_warn!("Stop announce failed (tracker will time out peer): {}", e);
            }
        }

        // ALWAYS transition to Stopped regardless of announce result
        *write_lock!(self.state) = FakerState::Stopped;
        let mut stats = write_lock!(self.stats);
        stats.state = FakerState::Stopped;
        stats.is_idling = false;
        stats.idling_reason = None;

        Ok(())
    }

    /// Update the fake stats (call this periodically)
    pub async fn update(&mut self) -> Result<()> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);
        self.last_update = now;

        let mut stats = write_lock!(self.stats);

        // Calculate and apply rates
        let (upload_rate, download_rate) = self.calculate_current_rates(&mut stats);
        self.update_rate_stats(&mut stats, upload_rate, download_rate);

        // Update transfer amounts
        let upload_delta = (upload_rate * 1024.0 * elapsed.as_secs_f64()) as u64;
        let download_delta = (download_rate * 1024.0 * elapsed.as_secs_f64()) as u64;

        log_trace!(
            "Update: elapsed={:.2}s, upload_rate={:.2} KB/s, download_rate={:.2} KB/s, upload_delta={} bytes",
            elapsed.as_secs_f64(),
            upload_rate,
            download_rate,
            upload_delta
        );

        let completed = self.update_transfer_stats(&mut stats, upload_delta, download_delta);

        if completed {
            drop(stats);
            if let Err(e) = self.on_completed().await {
                log_warn!("Completion announce failed, continuing: {}", e);
            }
            stats = write_lock!(self.stats);
        }

        // Update derived stats
        self.update_derived_stats(&mut stats, now);

        // Check stop conditions
        if self.check_stop_conditions(&stats) {
            log_info!("Stop condition met, stopping faker");
            drop(stats);
            self.stop().await?;
            return Ok(());
        }

        // Check if we need to announce
        if let Some(next_announce) = stats.next_announce {
            if now >= next_announce {
                drop(stats);
                if let Err(e) = self.periodic_announce().await {
                    log_warn!("Periodic announce failed, will retry next cycle: {}", e);
                    // Schedule a retry in 30s instead of waiting the full interval
                    let mut stats = write_lock!(self.stats);
                    stats.next_announce = Some(Instant::now() + Duration::from_secs(30));
                }
            }
        }

        Ok(())
    }

    /// Update only the stats without announcing to tracker (for live updates)
    pub async fn update_stats_only(&mut self) -> Result<()> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);
        self.last_update = now;

        // Periodic scrape for peer counts (if supported and enough time has passed)
        if self.scrape_supported && now.duration_since(self.last_scrape).as_secs() >= self.config.scrape_interval {
            match self.scrape().await {
                Ok(scrape_response) => {
                    let mut stats = write_lock!(self.stats);
                    stats.seeders = scrape_response.complete;
                    stats.leechers = scrape_response.incomplete;
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

        let mut stats = write_lock!(self.stats);

        // Calculate and apply rates
        let (upload_rate, download_rate) = self.calculate_current_rates(&mut stats);
        self.update_rate_stats(&mut stats, upload_rate, download_rate);

        // Update transfer amounts
        let upload_delta = (upload_rate * 1024.0 * elapsed.as_secs_f64()) as u64;
        let download_delta = (download_rate * 1024.0 * elapsed.as_secs_f64()) as u64;

        let completed = self.update_transfer_stats(&mut stats, upload_delta, download_delta);

        if completed {
            drop(stats);
            if let Err(e) = self.on_completed().await {
                log_warn!("Completion announce failed, continuing: {}", e);
            }
            stats = write_lock!(self.stats);
        }

        // Update derived stats
        self.update_derived_stats(&mut stats, now);

        // Check stop conditions
        if self.check_stop_conditions(&stats) {
            log_info!("Stop condition met, stopping faker");
            drop(stats);
            self.stop().await?;
            return Ok(());
        }

        // NOTE: We don't check for periodic announce here - that's handled by update()

        Ok(())
    }

    /// Get current stats
    pub async fn get_stats(&self) -> FakerStats {
        read_lock!(self.stats).clone()
    }

    /// Get torrent info
    pub fn get_torrent(&self) -> &TorrentInfo {
        &self.torrent
    }

    /// Send an announce to the tracker
    async fn announce(&mut self, event: TrackerEvent) -> Result<AnnounceResponse> {
        let stats = read_lock!(self.stats);

        log_debug!(
            "Preparing announce: event={:?}, uploaded={}, downloaded={}, left={}",
            event,
            stats.uploaded,
            stats.downloaded,
            stats.left
        );

        let request = AnnounceRequest {
            info_hash: self.torrent.info_hash,
            peer_id: self.peer_id.clone(),
            port: self.config.port,
            uploaded: stats.uploaded,
            downloaded: stats.downloaded,
            left: stats.left,
            compact: true,
            no_peer_id: false,
            event,
            ip: None,
            numwant: Some(self.config.num_want),
            key: Some(self.key.clone()),
            tracker_id: self.tracker_id.clone(),
        };

        drop(stats); // Release lock before async call

        let response = self
            .tracker_client
            .announce(self.torrent.get_tracker_url(), &request)
            .await?;

        Ok(response)
    }

    /// Periodic announce (no event)
    async fn periodic_announce(&mut self) -> Result<()> {
        log_info!("Sending periodic announce");

        let response = self.announce(TrackerEvent::None).await?;

        // Update interval if changed
        self.announce_interval = Duration::from_secs(response.interval as u64);

        // Update stats
        let mut stats = write_lock!(self.stats);
        stats.seeders = response.complete;
        stats.leechers = response.incomplete;
        stats.last_announce = Some(Instant::now());
        stats.next_announce = Some(Instant::now() + self.announce_interval);
        stats.announce_count += 1;

        log_info!(
            "Periodic announce complete. Seeders: {}, Leechers: {}",
            response.complete,
            response.incomplete
        );

        Ok(())
    }

    /// Handle completion event
    async fn on_completed(&mut self) -> Result<()> {
        log_info!("Torrent completed! Sending completed event");

        let response = self.announce(TrackerEvent::Completed).await?;

        // Update stats (state stays Running — faker continues seeding after completion)
        let mut stats = write_lock!(self.stats);
        stats.seeders = response.complete;
        stats.leechers = response.incomplete;
        stats.announce_count += 1;

        Ok(())
    }

    /// Scrape the tracker for stats
    pub async fn scrape(&self) -> Result<crate::protocol::ScrapeResponse> {
        log_info!("Scraping tracker");

        let response = self
            .tracker_client
            .scrape(self.torrent.get_tracker_url(), &self.torrent.info_hash)
            .await?;

        log_info!(
            "Scrape complete. Seeders: {}, Leechers: {}, Downloaded: {}",
            response.complete,
            response.incomplete,
            response.downloaded
        );

        Ok(response)
    }

    /// Pause the faker
    pub async fn pause(&mut self) -> Result<()> {
        log_info!("Pausing ratio faker");
        *write_lock!(self.state) = FakerState::Paused;
        let mut stats = write_lock!(self.stats);
        stats.state = FakerState::Paused;
        stats.is_idling = false;
        stats.idling_reason = None;
        Ok(())
    }

    /// Resume the faker
    pub async fn resume(&mut self) -> Result<()> {
        log_info!("Resuming ratio faker");
        *write_lock!(self.state) = FakerState::Running;
        write_lock!(self.stats).state = FakerState::Running;
        self.last_update = Instant::now(); // Reset to avoid large delta
        Ok(())
    }

    /// Calculate current upload and download rates with progressive and random adjustments
    /// Also updates idle state in stats based on peer availability
    fn calculate_current_rates(&self, stats: &mut FakerStats) -> (f64, f64) {
        let base_upload_rate = if self.config.progressive_rates {
            self.calculate_progressive_rate(
                self.config.upload_rate,
                self.config.target_upload_rate.unwrap_or(self.config.upload_rate),
                stats.elapsed_time.as_secs(),
                self.config.progressive_duration,
            )
        } else {
            self.config.upload_rate
        };

        let base_download_rate = if self.config.progressive_rates {
            self.calculate_progressive_rate(
                self.config.download_rate,
                self.config.target_download_rate.unwrap_or(self.config.download_rate),
                stats.elapsed_time.as_secs(),
                self.config.progressive_duration,
            )
        } else {
            self.config.download_rate
        };

        // Apply randomization
        let mut upload_rate = self.apply_randomization(base_upload_rate);
        let mut download_rate = self.apply_randomization(base_download_rate);

        // Reset idle state
        stats.is_idling = false;
        stats.idling_reason = None;

        // No download if we're complete (left == 0 means nothing left to download)
        if stats.left == 0 {
            download_rate = 0.0;
        }

        // Idle when no leechers (for seeders) - only after first announce
        // This keeps us connected to the tracker but with 0 upload rate
        if self.config.idle_when_no_leechers && stats.leechers == 0 && stats.announce_count > 0 {
            log_debug!(
                "Idling: no leechers to upload to (leechers={}, announce_count={})",
                stats.leechers,
                stats.announce_count
            );
            upload_rate = 0.0;
            stats.is_idling = true;
            stats.idling_reason = Some("no_leechers".to_string());
        }

        // Idle when no seeders (for leechers) - only after first announce and if we still need data
        // This keeps us connected to the tracker but with 0 download rate
        if self.config.idle_when_no_seeders && stats.seeders == 0 && stats.left > 0 && stats.announce_count > 0 {
            log_debug!(
                "Idling: no seeders to download from (seeders={}, left={}, announce_count={})",
                stats.seeders,
                stats.left,
                stats.announce_count
            );
            download_rate = 0.0;
            // Only set idling if not already idling for another reason
            if !stats.is_idling {
                stats.is_idling = true;
                stats.idling_reason = Some("no_seeders".to_string());
            }
        }

        (upload_rate, download_rate)
    }

    /// Apply randomization to a rate if enabled
    fn apply_randomization(&self, base_rate: f64) -> f64 {
        if self.config.randomize_rates {
            let mut rng = rand::rng();
            let range = self.config.random_range_percent / 100.0;
            let variation = 1.0 + (rng.random::<f64>() * (range * 2.0) - range);
            base_rate * variation
        } else {
            base_rate
        }
    }

    /// Update rate statistics and history
    fn update_rate_stats(&self, stats: &mut FakerStats, upload_rate: f64, download_rate: f64) {
        stats.current_upload_rate = upload_rate;
        stats.current_download_rate = download_rate;

        // Record timestamp for this data point (Unix millis)
        let timestamp = Self::current_timestamp_millis();
        Self::add_to_history_u64(&mut stats.history_timestamps, timestamp, 60);

        Self::add_to_history(&mut stats.upload_rate_history, upload_rate, 60);
        Self::add_to_history(&mut stats.download_rate_history, download_rate, 60);
    }

    /// Update transfer stats (uploaded, downloaded, left). Returns true if just completed.
    fn update_transfer_stats(&self, stats: &mut FakerStats, upload_delta: u64, download_delta: u64) -> bool {
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
    fn update_derived_stats(&self, stats: &mut FakerStats, now: Instant) {
        // Cumulative ratio (for display in Total Stats)
        let current_ratio = if self.torrent.total_size > 0 {
            stats.uploaded as f64 / self.torrent.total_size as f64
        } else {
            0.0
        };
        stats.ratio = current_ratio;
        Self::add_to_history(&mut stats.ratio_history, current_ratio, 60);

        // Session ratio (for stop conditions) = session_uploaded / torrent_size
        stats.session_ratio = if self.torrent.total_size > 0 {
            stats.session_uploaded as f64 / self.torrent.total_size as f64
        } else {
            0.0
        };

        stats.elapsed_time = now.duration_since(self.start_time);

        // Torrent completion percentage
        stats.torrent_completion = if self.torrent.total_size > 0 {
            ((self.torrent.total_size - stats.left) as f64 / self.torrent.total_size as f64) * 100.0
        } else {
            100.0
        };

        let elapsed_secs = stats.elapsed_time.as_secs_f64();
        if elapsed_secs > 0.0 {
            stats.average_upload_rate = (stats.session_uploaded as f64 / 1024.0) / elapsed_secs;
            stats.average_download_rate = (stats.session_downloaded as f64 / 1024.0) / elapsed_secs;
        }

        self.update_progress_and_eta(stats);
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
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64
        }
        #[cfg(target_arch = "wasm32")]
        {
            js_sys::Date::now() as u64
        }
    }

    fn check_stop_conditions(&self, stats: &FakerStats) -> bool {
        // Check ratio target (use session ratio, not cumulative)
        if let Some(target_ratio) = self.config.stop_at_ratio {
            if stats.session_ratio >= target_ratio - 0.001 {
                log_info!(
                    "Target ratio reached: {:.3} >= {:.3} (session)",
                    stats.session_ratio,
                    target_ratio
                );
                return true;
            }
        }

        // Check uploaded target (session uploaded, not total)
        if let Some(target_uploaded) = self.config.stop_at_uploaded {
            if stats.session_uploaded >= target_uploaded {
                log_info!(
                    "Target uploaded reached: {} >= {} bytes (session)",
                    stats.session_uploaded,
                    target_uploaded
                );
                return true;
            }
        }

        // Check downloaded target (session downloaded, not total)
        if let Some(target_downloaded) = self.config.stop_at_downloaded {
            if stats.session_downloaded >= target_downloaded {
                log_info!(
                    "Target downloaded reached: {} >= {} bytes (session)",
                    stats.session_downloaded,
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
        start_rate + (target_rate - start_rate) * progress
    }

    /// Update progress percentages and ETAs
    fn update_progress_and_eta(&self, stats: &mut FakerStats) {
        // Upload progress (based on session uploaded)
        if let Some(target) = self.config.stop_at_uploaded {
            stats.upload_progress = ((stats.session_uploaded as f64 / target as f64) * 100.0).min(100.0);

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
        if let Some(target) = self.config.stop_at_downloaded {
            stats.download_progress = ((stats.session_downloaded as f64 / target as f64) * 100.0).min(100.0);
        } else {
            stats.download_progress = 0.0;
        }

        // Ratio progress (use session ratio for progress tracking)
        if let Some(target_ratio) = self.config.stop_at_ratio {
            stats.ratio_progress = ((stats.session_ratio / target_ratio) * 100.0).min(100.0);

            // Calculate ETA for ratio (based on session stats)
            if stats.average_upload_rate > 0.0 && self.torrent.total_size > 0 {
                let target_session_uploaded = (target_ratio * self.torrent.total_size as f64) as u64;
                let remaining = target_session_uploaded.saturating_sub(stats.session_uploaded);
                let eta_secs = (remaining as f64 / 1024.0) / stats.average_upload_rate;
                stats.eta_ratio = Some(Duration::from_secs_f64(eta_secs));
            }
        } else {
            stats.ratio_progress = 0.0;
            stats.eta_ratio = None;
        }

        // Seed time progress
        if let Some(target_time) = self.config.stop_at_seed_time {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_faker_config_default() {
        let config = FakerConfig::default();
        assert_eq!(config.upload_rate, 50.0);
        assert_eq!(config.download_rate, 100.0);
    }

    #[test]
    fn test_preset_settings_to_faker_config_defaults() {
        let preset = PresetSettings::default();
        let config: FakerConfig = preset.into();

        assert_eq!(config.upload_rate, 50.0);
        assert_eq!(config.download_rate, 100.0);
        assert_eq!(config.port, 6881);
        assert_eq!(config.client_type, ClientType::QBittorrent);
        assert_eq!(config.completion_percent, 100.0);
        assert!(config.randomize_rates);
        assert_eq!(config.random_range_percent, 20.0);
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
}
