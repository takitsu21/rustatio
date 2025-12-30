use crate::log_info;
use crate::protocol::{AnnounceRequest, AnnounceResponse, TrackerClient, TrackerError, TrackerEvent};
use crate::torrent::{ClientConfig, ClientType, TorrentInfo};
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
    #[serde(default = "default_true")]
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

fn default_true() -> bool {
    true
}

fn default_progressive_duration() -> u64 {
    3600 // 1 hour
}

fn default_random_range() -> f64 {
    20.0
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
    Running,
    Paused,
    Stopped,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FakerStats {
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub seeders: i64,
    pub leechers: i64,
    pub ratio: f64,
    pub elapsed_time: Duration,
    pub state: FakerState,
    #[serde(skip)]
    pub last_announce: Option<Instant>,
    #[serde(skip)]
    pub next_announce: Option<Instant>,

    // Session stats
    pub session_uploaded: u64,
    pub session_downloaded: u64,
    pub current_upload_rate: f64,   // KB/s
    pub current_download_rate: f64, // KB/s
    pub average_upload_rate: f64,   // KB/s
    pub average_download_rate: f64, // KB/s

    // Progress tracking
    pub upload_progress: f64,    // 0-100 % (if stop_at_uploaded is set)
    pub download_progress: f64,  // 0-100 % (if stop_at_downloaded is set)
    pub ratio_progress: f64,     // 0-100 % (if stop_at_ratio is set)
    pub seed_time_progress: f64, // 0-100 % (if stop_at_seed_time is set)

    // ETA
    pub eta_ratio: Option<Duration>,
    pub eta_uploaded: Option<Duration>,
    pub eta_seed_time: Option<Duration>,

    // Rate history (last 60 data points for graphs)
    pub upload_rate_history: Vec<f64>,
    pub download_rate_history: Vec<f64>,
    pub ratio_history: Vec<f64>,
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
}

impl RatioFaker {
    pub fn new(torrent: TorrentInfo, config: FakerConfig) -> Result<Self> {
        // Create client configuration
        let client_config = ClientConfig::get(config.client_type.clone(), config.client_version.clone());

        // Generate session identifiers
        let peer_id = client_config.generate_peer_id();
        let key = ClientConfig::generate_key();

        // Create tracker client
        let tracker_client =
            TrackerClient::new(client_config.clone()).map_err(|e| FakerError::ConfigError(e.to_string()))?;

        // Calculate initial stats
        let completion = config.completion_percent.clamp(0.0, 100.0) / 100.0;
        let downloaded = config.initial_downloaded + (torrent.total_size as f64 * completion) as u64;
        let left = torrent.total_size.saturating_sub(downloaded);

        let stats = FakerStats {
            uploaded: config.initial_uploaded,
            downloaded,
            left,
            seeders: 0,
            leechers: 0,
            ratio: if downloaded > 0 {
                config.initial_uploaded as f64 / downloaded as f64
            } else {
                0.0
            },
            elapsed_time: Duration::from_secs(0),
            state: FakerState::Idle,
            last_announce: None,
            next_announce: None,
            session_uploaded: 0,
            session_downloaded: 0,
            current_upload_rate: 0.0,
            current_download_rate: 0.0,
            average_upload_rate: 0.0,
            average_download_rate: 0.0,
            upload_progress: 0.0,
            download_progress: 0.0,
            ratio_progress: 0.0,
            seed_time_progress: 0.0,
            eta_ratio: None,
            eta_uploaded: None,
            eta_seed_time: None,
            upload_rate_history: Vec::new(),
            download_rate_history: Vec::new(),
            ratio_history: Vec::new(),
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(RatioFaker {
                torrent,
                config,
                tracker_client,
                state: Arc::new(RwLock::new(FakerState::Idle)),
                stats: Arc::new(RwLock::new(stats)),
                peer_id,
                key,
                tracker_id: None,
                start_time: Instant::now(),
                last_update: Instant::now(),
                announce_interval: Duration::from_secs(1800), // Default 30 minutes
            })
        }

        #[cfg(target_arch = "wasm32")]
        {
            Ok(RatioFaker {
                torrent,
                config,
                tracker_client,
                state: RefCell::new(FakerState::Idle),
                stats: RefCell::new(stats),
                peer_id,
                key,
                tracker_id: None,
                start_time: Instant::now(),
                last_update: Instant::now(),
                announce_interval: Duration::from_secs(1800), // Default 30 minutes
            })
        }
    }

    /// Start the ratio faking session
    pub async fn start(&mut self) -> Result<()> {
        log_info!("Starting ratio faker for torrent: {}", self.torrent.name);

        // Update state
        *write_lock!(self.state) = FakerState::Running;
        self.start_time = Instant::now();
        self.last_update = Instant::now();

        // Send started event
        let response = self.announce(TrackerEvent::Started).await?;

        // Update announce interval
        self.announce_interval = Duration::from_secs(response.interval as u64);

        // Store tracker ID if provided
        self.tracker_id = response.tracker_id;

        // Update stats with tracker response
        let mut stats = write_lock!(self.stats);
        stats.state = FakerState::Running; // Ensure state is synced
        stats.seeders = response.complete;
        stats.leechers = response.incomplete;
        stats.last_announce = Some(Instant::now());
        stats.next_announce = Some(Instant::now() + self.announce_interval);

        log_info!(
            "Started successfully. Seeders: {}, Leechers: {}, Interval: {}s",
            response.complete,
            response.incomplete,
            response.interval
        );

        Ok(())
    }

    /// Stop the ratio faking session
    pub async fn stop(&mut self) -> Result<()> {
        log_info!("Stopping ratio faker");

        // Send stopped event
        self.announce(TrackerEvent::Stopped).await?;

        // Update state
        *write_lock!(self.state) = FakerState::Stopped;

        // CRITICAL: Also update the state in stats so frontend can detect the stop
        write_lock!(self.stats).state = FakerState::Stopped;

        Ok(())
    }

    /// Update the fake stats (call this periodically)
    pub async fn update(&mut self) -> Result<()> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);
        self.last_update = now;

        let mut stats = write_lock!(self.stats);

        // Calculate progressive rates if enabled
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

        // Apply randomization if enabled
        let upload_rate = if self.config.randomize_rates {
            let mut rng = rand::rng();
            let range = self.config.random_range_percent / 100.0;
            let variation = 1.0 + (rng.random::<f64>() * (range * 2.0) - range);
            base_upload_rate * variation
        } else {
            base_upload_rate
        };

        let download_rate = if self.config.randomize_rates {
            let mut rng = rand::rng();
            let range = self.config.random_range_percent / 100.0;
            let variation = 1.0 + (rng.random::<f64>() * (range * 2.0) - range);
            base_download_rate * variation
        } else {
            base_download_rate
        };

        // Store current rates
        stats.current_upload_rate = upload_rate;
        stats.current_download_rate = download_rate;

        // Update rate history (keep last 60 points)
        stats.upload_rate_history.push(upload_rate);
        stats.download_rate_history.push(download_rate);
        if stats.upload_rate_history.len() > 60 {
            stats.upload_rate_history.remove(0);
        }
        if stats.download_rate_history.len() > 60 {
            stats.download_rate_history.remove(0);
        }

        // Calculate bytes to add based on rates
        let upload_delta = (upload_rate * 1024.0 * elapsed.as_secs_f64()) as u64;
        let download_delta = (download_rate * 1024.0 * elapsed.as_secs_f64()) as u64;

        // Update uploaded
        stats.uploaded += upload_delta;
        stats.session_uploaded += upload_delta;

        // Update downloaded (but don't exceed total size)
        if stats.left > 0 {
            let actual_download = download_delta.min(stats.left);
            stats.downloaded += actual_download;
            stats.session_downloaded += actual_download;
            stats.left = stats.left.saturating_sub(actual_download);

            // Check if we just completed
            if stats.left == 0 {
                drop(stats); // Release lock before async call
                self.on_completed().await?;
                stats = write_lock!(self.stats); // Re-acquire
            }
        }

        // Update ratio
        let current_ratio = if self.torrent.total_size > 0 {
            stats.uploaded as f64 / self.torrent.total_size as f64
        } else {
            0.0
        };
        stats.ratio = current_ratio;

        // Update ratio history (keep last 60 points)
        stats.ratio_history.push(current_ratio);
        if stats.ratio_history.len() > 60 {
            stats.ratio_history.remove(0);
        }

        // Update elapsed time
        stats.elapsed_time = now.duration_since(self.start_time);

        // Calculate average rates
        let elapsed_secs = stats.elapsed_time.as_secs_f64();
        if elapsed_secs > 0.0 {
            stats.average_upload_rate = (stats.session_uploaded as f64 / 1024.0) / elapsed_secs;
            stats.average_download_rate = (stats.session_downloaded as f64 / 1024.0) / elapsed_secs;
        }

        // Update progress and ETAs
        self.update_progress_and_eta(&mut stats);

        // Check stop conditions
        let should_stop = self.check_stop_conditions(&stats);
        if should_stop {
            log_info!("Stop condition met, stopping faker");
            drop(stats);
            self.stop().await?;
            return Ok(());
        }

        // Check if we need to announce
        if let Some(next_announce) = stats.next_announce {
            if now >= next_announce {
                drop(stats); // Release lock before async call
                self.periodic_announce().await?;
            }
        }

        Ok(())
    }

    /// Update only the stats without announcing to tracker (for live updates)
    pub async fn update_stats_only(&mut self) -> Result<()> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);
        self.last_update = now;

        let mut stats = write_lock!(self.stats);

        // Calculate progressive rates if enabled
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

        // Apply randomization if enabled
        let upload_rate = if self.config.randomize_rates {
            let mut rng = rand::rng();
            let range = self.config.random_range_percent / 100.0;
            let variation = 1.0 + (rng.random::<f64>() * (range * 2.0) - range);
            base_upload_rate * variation
        } else {
            base_upload_rate
        };

        let download_rate = if self.config.randomize_rates {
            let mut rng = rand::rng();
            let range = self.config.random_range_percent / 100.0;
            let variation = 1.0 + (rng.random::<f64>() * (range * 2.0) - range);
            base_download_rate * variation
        } else {
            base_download_rate
        };

        // Store current rates
        stats.current_upload_rate = upload_rate;
        stats.current_download_rate = download_rate;

        // Update rate history (keep last 60 points)
        stats.upload_rate_history.push(upload_rate);
        stats.download_rate_history.push(download_rate);
        if stats.upload_rate_history.len() > 60 {
            stats.upload_rate_history.remove(0);
        }
        if stats.download_rate_history.len() > 60 {
            stats.download_rate_history.remove(0);
        }

        // Calculate bytes to add based on rates
        let upload_delta = (upload_rate * 1024.0 * elapsed.as_secs_f64()) as u64;
        let download_delta = (download_rate * 1024.0 * elapsed.as_secs_f64()) as u64;

        // Update uploaded
        stats.uploaded += upload_delta;
        stats.session_uploaded += upload_delta;

        // Update downloaded (but don't exceed total size)
        if stats.left > 0 {
            let actual_download = download_delta.min(stats.left);
            stats.downloaded += actual_download;
            stats.session_downloaded += actual_download;
            stats.left = stats.left.saturating_sub(actual_download);

            // Check if we just completed
            if stats.left == 0 {
                drop(stats); // Release lock before async call
                self.on_completed().await?;
                stats = write_lock!(self.stats); // Re-acquire
            }
        }

        // Update ratio
        let current_ratio = if self.torrent.total_size > 0 {
            stats.uploaded as f64 / self.torrent.total_size as f64
        } else {
            0.0
        };
        stats.ratio = current_ratio;

        // Update ratio history (keep last 60 points)
        stats.ratio_history.push(current_ratio);
        if stats.ratio_history.len() > 60 {
            stats.ratio_history.remove(0);
        }

        // Update elapsed time
        stats.elapsed_time = now.duration_since(self.start_time);

        // Calculate average rates
        let elapsed_secs = stats.elapsed_time.as_secs_f64();
        if elapsed_secs > 0.0 {
            stats.average_upload_rate = (stats.session_uploaded as f64 / 1024.0) / elapsed_secs;
            stats.average_download_rate = (stats.session_downloaded as f64 / 1024.0) / elapsed_secs;
        }

        // Update progress and ETAs
        self.update_progress_and_eta(&mut stats);

        // Check stop conditions
        let should_stop = self.check_stop_conditions(&stats);
        if should_stop {
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

        // Update state
        *write_lock!(self.state) = FakerState::Completed;

        // Update stats
        let mut stats = write_lock!(self.stats);
        stats.state = FakerState::Completed; // CRITICAL: Update state in stats too
        stats.seeders = response.complete;
        stats.leechers = response.incomplete;

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
        write_lock!(self.stats).state = FakerState::Paused;
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

    /// Check if any stop conditions are met
    fn check_stop_conditions(&self, stats: &FakerStats) -> bool {
        // Check ratio target (use a small epsilon for floating point comparison)
        if let Some(target_ratio) = self.config.stop_at_ratio {
            if stats.ratio >= target_ratio - 0.001 {
                log_info!("Target ratio reached: {:.3} >= {:.3}", stats.ratio, target_ratio);
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

        // Ratio progress
        if let Some(target_ratio) = self.config.stop_at_ratio {
            stats.ratio_progress = ((stats.ratio / target_ratio) * 100.0).min(100.0);

            // Calculate ETA for ratio
            if stats.average_upload_rate > 0.0 && stats.downloaded > 0 {
                let target_uploaded = (target_ratio * stats.downloaded as f64) as u64;
                let remaining = target_uploaded.saturating_sub(stats.uploaded);
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
}
