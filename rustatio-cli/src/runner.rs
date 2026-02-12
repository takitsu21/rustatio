use crate::cli::ClientArg;
use crate::json::{
    AnnounceEvent, AnnounceType, InputCommand, OutputEvent, ScrapeEvent, StartedEvent, StatsEvent, StopReason,
    StoppedEvent,
};
use crate::session::Session;
use anyhow::{Context, Result};
use chrono::Utc;
use rustatio_core::{ClientConfig, ClientType, FakerConfig, FakerState, RatioFaker, TorrentInfo};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

/// Configuration for the runner
#[allow(dead_code)]
pub struct RunnerConfig {
    pub torrent_path: std::path::PathBuf,
    pub client: ClientArg,
    pub client_version: Option<String>,
    pub upload_rate: f64,
    pub download_rate: f64,
    pub port: u16,
    pub completion: f64,
    pub initial_uploaded: u64,
    pub initial_downloaded: u64,
    pub stop_ratio: Option<f64>,
    pub stop_uploaded: Option<f64>,
    pub stop_downloaded: Option<f64>,
    pub stop_time: Option<f64>,
    pub idle_when_no_leechers: bool,
    pub idle_when_no_seeders: bool,
    pub no_randomize: bool,
    pub random_range: f64,
    pub progressive: bool,
    pub target_upload: Option<f64>,
    pub target_download: Option<f64>,
    pub progressive_duration: f64,
    pub json_mode: bool,
    pub stats_interval: u64,
    pub save_session: bool,
    pub info_hash: String,
    pub torrent_name: String,
    pub torrent_size: u64,
}

/// Internal command for controlling the runner
#[derive(Debug)]
pub enum RunnerCommand {
    Pause,
    Resume,
    Stop,
    Scrape,
    Stats,
    Shutdown,
}

/// Run the faker in JSON mode
pub async fn run_json_mode(config: RunnerConfig) -> Result<()> {
    // Emit init event
    OutputEvent::init().emit();

    // Load torrent
    let torrent = load_torrent(&config.torrent_path)?;
    OutputEvent::TorrentLoaded((&torrent).into()).emit();

    // Create faker config
    let faker_config = create_faker_config(&config);

    // Get client info for started event
    let client_type: ClientType = config.client.into();
    let client_config = ClientConfig::get(client_type.clone(), config.client_version.clone());

    // Create faker
    let mut faker =
        RatioFaker::new(torrent, faker_config).map_err(|e| anyhow::anyhow!("Failed to create faker: {}", e))?;

    // Start faker
    faker
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start faker: {}", e))?;

    // Emit started event
    OutputEvent::Started(StartedEvent {
        peer_id: client_config.generate_peer_id(),
        client: format!("{:?}", client_type),
        client_version: client_config.version.clone(),
        port: config.port,
        timestamp: Utc::now(),
    })
    .emit();

    // Emit initial announce event
    let stats = faker.get_stats().await;
    OutputEvent::Announce(AnnounceEvent {
        announce_type: AnnounceType::Started,
        seeders: stats.seeders,
        leechers: stats.leechers,
        interval: 1800, // Default, will be updated
        timestamp: Utc::now(),
    })
    .emit();

    // Setup channels
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<RunnerCommand>(32);

    // Setup shutdown flag
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    // Setup Ctrl+C handler
    let cmd_tx_ctrlc = cmd_tx.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            shutdown_clone.store(true, Ordering::SeqCst);
            let _ = cmd_tx_ctrlc.send(RunnerCommand::Shutdown).await;
        }
    });

    // Setup stdin reader for commands
    let cmd_tx_stdin = cmd_tx.clone();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let reader = BufReader::new(stdin.lock());

        for line in reader.lines().map_while(Result::ok) {
            if let Ok(cmd) = InputCommand::parse(&line) {
                let runner_cmd = match cmd {
                    InputCommand::Pause => RunnerCommand::Pause,
                    InputCommand::Resume => RunnerCommand::Resume,
                    InputCommand::Stop => RunnerCommand::Stop,
                    InputCommand::Scrape => RunnerCommand::Scrape,
                    InputCommand::Stats => RunnerCommand::Stats,
                };
                if cmd_tx_stdin.blocking_send(runner_cmd).is_err() {
                    break;
                }
            }
        }
    });

    // Main loop
    let mut stats_ticker = interval(Duration::from_secs(config.stats_interval));
    let mut stop_reason = StopReason::UserInterrupt;

    loop {
        tokio::select! {
            _ = stats_ticker.tick() => {
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }

                // Update stats
                if let Err(e) = faker.update().await {
                    OutputEvent::error(format!("Update error: {}", e)).emit();
                }

                let stats = faker.get_stats().await;

                // Check if stopped by stop condition
                if matches!(stats.state, FakerState::Stopped) {
                    stop_reason = determine_stop_reason(&config, &stats);
                    break;
                }

                // Emit stats event
                OutputEvent::Stats(StatsEvent::from(&stats)).emit();
            }

            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    RunnerCommand::Pause => {
                        if let Err(e) = faker.pause().await {
                            OutputEvent::error(format!("Pause error: {}", e)).emit();
                        } else {
                            OutputEvent::paused().emit();
                        }
                    }
                    RunnerCommand::Resume => {
                        if let Err(e) = faker.resume().await {
                            OutputEvent::error(format!("Resume error: {}", e)).emit();
                        } else {
                            OutputEvent::resumed().emit();
                        }
                    }
                    RunnerCommand::Stop => {
                        stop_reason = StopReason::UserCommand;
                        break;
                    }
                    RunnerCommand::Scrape => {
                        match faker.scrape().await {
                            Ok(response) => {
                                OutputEvent::Scrape(ScrapeEvent {
                                    seeders: response.complete,
                                    leechers: response.incomplete,
                                    downloaded: response.downloaded,
                                    timestamp: Utc::now(),
                                }).emit();
                            }
                            Err(e) => {
                                OutputEvent::error(format!("Scrape error: {}", e)).emit();
                            }
                        }
                    }
                    RunnerCommand::Stats => {
                        let stats = faker.get_stats().await;
                        OutputEvent::Stats(StatsEvent::from(&stats)).emit();
                    }
                    RunnerCommand::Shutdown => {
                        stop_reason = StopReason::UserInterrupt;
                        break;
                    }
                }
            }
        }
    }

    // Stop faker gracefully
    let final_stats = faker.get_stats().await;

    if let Err(e) = faker.stop().await {
        OutputEvent::error(format!("Stop error: {}", e)).emit();
    }

    // Save session if enabled
    if config.save_session {
        let client_type: ClientType = config.client.into();
        let mut session = Session::new(
            &config.info_hash,
            &config.torrent_name,
            &config.torrent_path.to_string_lossy(),
            config.torrent_size,
            &format!("{:?}", client_type),
            config.client_version.clone(),
        );
        session.upload_rate = config.upload_rate;
        session.download_rate = config.download_rate;
        session.port = config.port;
        session.completion_percent = final_stats.torrent_completion;
        session.stop_at_ratio = config.stop_ratio;
        session.stop_at_uploaded_gb = config.stop_uploaded;
        session.update(
            final_stats.uploaded,
            final_stats.downloaded,
            final_stats.elapsed_time.as_secs(),
        );

        if let Err(e) = session.save_session() {
            OutputEvent::error(format!("Failed to save session: {}", e)).emit();
        }
    }

    // Emit stopped event
    OutputEvent::Stopped(StoppedEvent {
        reason: stop_reason,
        final_uploaded: final_stats.uploaded,
        final_downloaded: final_stats.downloaded,
        final_ratio: final_stats.ratio,
        session_uploaded: final_stats.session_uploaded,
        session_ratio: final_stats.session_ratio,
        elapsed_secs: final_stats.elapsed_time.as_secs(),
        timestamp: Utc::now(),
    })
    .emit();

    Ok(())
}

/// Load torrent file from path
pub fn load_torrent(path: &Path) -> Result<TorrentInfo> {
    TorrentInfo::from_file(path).context("Failed to parse torrent file")
}

/// Create FakerConfig from RunnerConfig
pub fn create_faker_config(config: &RunnerConfig) -> FakerConfig {
    FakerConfig {
        upload_rate: config.upload_rate,
        download_rate: config.download_rate,
        port: config.port,
        client_type: config.client.into(),
        client_version: config.client_version.clone(),
        initial_uploaded: config.initial_uploaded,
        initial_downloaded: config.initial_downloaded,
        completion_percent: config.completion,
        num_want: 50,
        randomize_rates: !config.no_randomize,
        random_range_percent: config.random_range,
        stop_at_ratio: config.stop_ratio,
        stop_at_uploaded: config.stop_uploaded.map(|gb| (gb * 1024.0 * 1024.0 * 1024.0) as u64),
        stop_at_downloaded: config.stop_downloaded.map(|gb| (gb * 1024.0 * 1024.0 * 1024.0) as u64),
        stop_at_seed_time: config.stop_time.map(|hours| (hours * 3600.0) as u64),
        idle_when_no_leechers: config.idle_when_no_leechers,
        idle_when_no_seeders: config.idle_when_no_seeders,
        scrape_interval: 60,
        progressive_rates: config.progressive,
        target_upload_rate: config.target_upload,
        target_download_rate: config.target_download,
        progressive_duration: (config.progressive_duration * 3600.0) as u64,
    }
}

/// Determine why the faker stopped based on config and final stats
fn determine_stop_reason(config: &RunnerConfig, stats: &rustatio_core::FakerStats) -> StopReason {
    if let Some(target_ratio) = config.stop_ratio {
        if stats.session_ratio >= target_ratio - 0.001 {
            return StopReason::TargetRatio;
        }
    }

    if let Some(target_gb) = config.stop_uploaded {
        let target_bytes = (target_gb * 1024.0 * 1024.0 * 1024.0) as u64;
        if stats.session_uploaded >= target_bytes {
            return StopReason::TargetUploaded;
        }
    }

    if let Some(target_gb) = config.stop_downloaded {
        let target_bytes = (target_gb * 1024.0 * 1024.0 * 1024.0) as u64;
        if stats.session_downloaded >= target_bytes {
            return StopReason::TargetDownloaded;
        }
    }

    if let Some(target_hours) = config.stop_time {
        let target_secs = (target_hours * 3600.0) as u64;
        if stats.elapsed_time.as_secs() >= target_secs {
            return StopReason::TargetSeedTime;
        }
    }

    StopReason::UserInterrupt
}
