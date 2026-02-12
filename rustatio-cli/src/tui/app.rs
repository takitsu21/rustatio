use crate::json::{format_bytes, format_duration};
use crate::runner::RunnerConfig;
use crate::session::Session;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame, Terminal,
};
use rustatio_core::{ClientConfig, ClientType, FakerState, FakerStats, RatioFaker, TorrentInfo};
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration as StdDuration, Instant};
use tokio::time::{interval, Duration};

/// TUI Application state
pub struct App {
    pub torrent: TorrentInfo,
    pub client_type: ClientType,
    pub client_version: String,
    pub stats: Option<FakerStats>,
    pub status_message: Option<String>,
    pub should_quit: bool,

    // Config values to display
    pub completion: f64,
    pub upload_rate: f64,
    pub download_rate: f64,
    pub port: u16,

    // Stop conditions
    pub target_ratio: Option<f64>,
    pub target_uploaded: Option<f64>, // in GB
    pub target_time: Option<f64>,     // in hours

    // Track announce count to detect new announces
    pub last_announce_count: u32,
}

impl App {
    pub fn new(torrent: TorrentInfo, config: &RunnerConfig) -> Self {
        let client_type: ClientType = config.client.into();
        let client_config = ClientConfig::get(client_type.clone(), config.client_version.clone());

        App {
            torrent,
            client_type,
            client_version: client_config.version,
            stats: None,
            status_message: None,
            should_quit: false,
            completion: config.completion,
            upload_rate: config.upload_rate,
            download_rate: config.download_rate,
            port: config.port,
            target_ratio: config.stop_ratio,
            target_uploaded: config.stop_uploaded,
            target_time: config.stop_time,
            last_announce_count: 0,
        }
    }

    pub fn update_stats(&mut self, stats: FakerStats) {
        self.stats = Some(stats);
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }

    /// Check if any stop condition is set
    pub fn has_stop_condition(&self) -> bool {
        self.target_ratio.is_some() || self.target_uploaded.is_some() || self.target_time.is_some()
    }
}

/// Keyboard commands
#[derive(Debug)]
enum KeyCommand {
    Quit,
    Pause,
    Resume,
    Stop,
    Scrape,
}

/// Run the TUI mode
pub async fn run_tui_mode(config: RunnerConfig) -> Result<()> {
    // Load torrent
    let torrent = crate::runner::load_torrent(&config.torrent_path)?;

    // Create app state
    let mut app = App::new(torrent.clone(), &config);

    // Create faker config
    let faker_config = crate::runner::create_faker_config(&config);

    // Create faker
    let mut faker =
        RatioFaker::new(torrent, faker_config).map_err(|e| anyhow::anyhow!("Failed to create faker: {}", e))?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Start faker
    app.set_status("Starting...");
    terminal.draw(|f| ui(f, &app))?;

    if let Err(e) = faker.start().await {
        cleanup_terminal(&mut terminal)?;
        return Err(anyhow::anyhow!("Failed to start faker: {}", e));
    }

    app.set_status("Running");

    // Setup keyboard event channel - use std::sync::mpsc for thread communication
    let (key_tx, key_rx) = mpsc::channel::<KeyCommand>();

    // Spawn keyboard event reader thread
    thread::spawn(move || {
        loop {
            // Poll for events with a timeout
            if event::poll(StdDuration::from_millis(100)).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    if key.kind == KeyEventKind::Press {
                        let cmd = match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => Some(KeyCommand::Quit),
                            KeyCode::Char('p') => Some(KeyCommand::Pause),
                            KeyCode::Char('r') => Some(KeyCommand::Resume),
                            KeyCode::Char('x') => Some(KeyCommand::Stop),
                            KeyCode::Char('s') => Some(KeyCommand::Scrape),
                            _ => None,
                        };

                        if let Some(cmd) = cmd {
                            if key_tx.send(cmd).is_err() {
                                break; // Channel closed, exit thread
                            }
                        }
                    }
                }
            }
        }
    });

    // Main loop
    let mut stats_ticker = interval(Duration::from_millis(500));

    loop {
        // Check for keyboard commands (non-blocking)
        while let Ok(cmd) = key_rx.try_recv() {
            match cmd {
                KeyCommand::Quit => {
                    app.should_quit = true;
                    app.set_status("Quitting...");
                    break;
                }
                KeyCommand::Pause => {
                    if let Some(ref stats) = app.stats {
                        if matches!(stats.state, FakerState::Running) {
                            if let Err(e) = faker.pause().await {
                                app.set_status(format!("Pause failed: {}", e));
                            } else {
                                app.set_status("Paused - press [r] to resume");
                            }
                        }
                    }
                }
                KeyCommand::Resume => {
                    if let Some(ref stats) = app.stats {
                        if matches!(stats.state, FakerState::Paused) {
                            if let Err(e) = faker.resume().await {
                                app.set_status(format!("Resume failed: {}", e));
                            } else {
                                app.set_status("Resumed");
                            }
                        }
                    }
                }
                KeyCommand::Stop => {
                    app.set_status("Stopping...");
                    terminal.draw(|f| ui(f, &app))?;
                    if let Err(e) = faker.stop().await {
                        app.set_status(format!("Stop failed: {}", e));
                    } else {
                        app.set_status("Stopped");
                        app.should_quit = true;
                    }
                }
                KeyCommand::Scrape => {
                    app.set_status("Scraping tracker...");
                    terminal.draw(|f| ui(f, &app))?;
                    match faker.scrape().await {
                        Ok(resp) => {
                            app.set_status(format!(
                                "Scrape: {} seeders, {} leechers",
                                resp.complete, resp.incomplete
                            ));
                        }
                        Err(e) => {
                            app.set_status(format!("Scrape failed: {}", e));
                        }
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }

        // Wait for next tick
        stats_ticker.tick().await;

        // Get current stats first to check state
        let stats = faker.get_stats().await;

        // Only update if running (not paused)
        if matches!(stats.state, FakerState::Running) {
            // Use update() which handles periodic announces
            if let Err(e) = faker.update().await {
                app.set_status(format!("Update error: {}", e));
            }
        }

        // Get updated stats
        let stats = faker.get_stats().await;

        // Check if a new announce happened
        if stats.announce_count > app.last_announce_count {
            app.last_announce_count = stats.announce_count;
            app.set_status(format!(
                "Announced to tracker (#{}) - {} seeders, {} leechers",
                stats.announce_count, stats.seeders, stats.leechers
            ));
        }

        // Check if stopped by stop condition
        if matches!(stats.state, FakerState::Stopped) {
            app.update_stats(stats);
            app.set_status("Stopped");
            terminal.draw(|f| ui(f, &app))?;

            // Wait a moment then exit
            tokio::time::sleep(Duration::from_secs(2)).await;
            break;
        }

        app.update_stats(stats);
        terminal.draw(|f| ui(f, &app))?;
    }

    // Stop faker gracefully if not already stopped
    if !matches!(app.stats.as_ref().map(|s| &s.state), Some(FakerState::Stopped)) {
        app.set_status("Stopping...");
        terminal.draw(|f| ui(f, &app))?;
        let _ = faker.stop().await;
    }

    // Cleanup
    cleanup_terminal(&mut terminal)?;

    // Save session if enabled
    if config.save_session {
        if let Some(ref stats) = app.stats {
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
            session.completion_percent = config.completion;
            session.stop_at_ratio = config.stop_ratio;
            session.stop_at_uploaded_gb = config.stop_uploaded;
            session.update(stats.uploaded, stats.downloaded, stats.elapsed_time.as_secs());

            if let Err(e) = session.save_session() {
                eprintln!("Warning: Failed to save session: {}", e);
            } else {
                println!("Session saved. Use --resume to continue later.");
            }
        }
    }

    // Print final stats
    if let Some(stats) = app.stats {
        println!("\nFinal Statistics:");
        println!("  Uploaded:   {}", format_bytes(stats.uploaded));
        println!("  Downloaded: {}", format_bytes(stats.downloaded));
        println!("  Ratio:      {:.3}", stats.ratio);
        println!("  Session:    {}", format_duration(stats.elapsed_time.as_secs()));
    }

    Ok(())
}

fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Render the UI
fn ui(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // Determine layout based on whether we have stop conditions
    let has_progress = app.has_stop_condition();

    // Create main layout
    let constraints = if has_progress {
        vec![
            Constraint::Length(3), // Header
            Constraint::Length(6), // Torrent info (expanded)
            Constraint::Length(3), // Status bar
            Constraint::Length(8), // Stats (expanded)
            Constraint::Length(3), // Tracker/Announce info
            Constraint::Length(5), // Progress section
            Constraint::Min(3),    // Help
        ]
    } else {
        vec![
            Constraint::Length(3), // Header
            Constraint::Length(6), // Torrent info (expanded)
            Constraint::Length(3), // Status bar
            Constraint::Length(8), // Stats (expanded)
            Constraint::Length(3), // Tracker/Announce info
            Constraint::Min(3),    // Help
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(size);

    // Header
    let header = Paragraph::new(format!(
        " rustatio v{} - BitTorrent Ratio Faker",
        env!("CARGO_PKG_VERSION")
    ))
    .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Torrent info (expanded with more details)
    render_torrent_info(frame, app, chunks[1]);

    // Status bar with status message
    render_status_bar(frame, app, chunks[2]);

    // Stats
    render_stats(frame, app, chunks[3]);

    // Tracker/Announce info
    render_tracker_info(frame, app, chunks[4]);

    // Progress section (if stop conditions set)
    if has_progress {
        render_progress(frame, app, chunks[5]);
        render_help(frame, chunks[6]);
    } else {
        render_help(frame, chunks[5]);
    }
}

fn render_torrent_info(frame: &mut Frame, app: &App, area: Rect) {
    let info_hash = app.torrent.info_hash_hex();
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Torrent: ", Style::default().fg(Color::Gray)),
            Span::styled(&app.torrent.name, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Tracker: ", Style::default().fg(Color::Gray)),
            Span::styled(&app.torrent.announce, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("Size:    ", Style::default().fg(Color::Gray)),
            Span::styled(format_bytes(app.torrent.total_size), Style::default().fg(Color::White)),
            Span::raw("   "),
            Span::styled("Hash: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{}…", &info_hash[..16]), Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("Client:  ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:?} {}", app.client_type, app.client_version),
                Style::default().fg(Color::Green),
            ),
            Span::raw("   "),
            Span::styled("Port: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{}", app.port), Style::default().fg(Color::White)),
            Span::raw("   "),
            Span::styled("Completion: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.1}%", app.completion),
                if app.completion >= 100.0 {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Yellow)
                },
            ),
        ]),
    ];

    // Add configured rates
    lines.push(Line::from(vec![
        Span::styled("Rates:   ", Style::default().fg(Color::Gray)),
        Span::styled("↑ ", Style::default().fg(Color::Green)),
        Span::styled(
            format!("{:.0} KB/s", app.upload_rate),
            Style::default().fg(Color::White),
        ),
        Span::raw("   "),
        Span::styled("↓ ", Style::default().fg(Color::Blue)),
        Span::styled(
            format!("{:.0} KB/s", app.download_rate),
            Style::default().fg(Color::White),
        ),
    ]));

    let torrent_info = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Torrent "));
    frame.render_widget(torrent_info, area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let (status_text, status_color) = if let Some(ref stats) = app.stats {
        match stats.state {
            FakerState::Running => ("● Running", Color::Green),
            FakerState::Starting => ("◌ Starting", Color::Yellow),
            FakerState::Stopping => ("◼ Stopping", Color::Yellow),
            FakerState::Paused => ("⏸ Paused", Color::Yellow),
            FakerState::Stopped => ("■ Stopped", Color::Red),
            FakerState::Idle => ("○ Idle", Color::Gray),
        }
    } else {
        ("○ Initializing", Color::Gray)
    };

    let seeders = app.stats.as_ref().map(|s| s.seeders).unwrap_or(0);
    let leechers = app.stats.as_ref().map(|s| s.leechers).unwrap_or(0);

    let mut status_spans = vec![
        Span::styled(format!(" {}", status_text), Style::default().fg(status_color)),
        Span::raw("   "),
        Span::styled("Peers: ", Style::default().fg(Color::Gray)),
        Span::styled(format!("{}", seeders), Style::default().fg(Color::Green)),
        Span::styled(" S ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", leechers), Style::default().fg(Color::Yellow)),
        Span::styled(" L", Style::default().fg(Color::DarkGray)),
    ];

    // Add status message if present
    if let Some(ref msg) = app.status_message {
        status_spans.push(Span::raw("   "));
        status_spans.push(Span::styled(format!("[{}]", msg), Style::default().fg(Color::Magenta)));
    }

    let status_line = Line::from(status_spans);
    let status_bar = Paragraph::new(status_line).block(Block::default().borders(Borders::ALL).title(" Status "));
    frame.render_widget(status_bar, area);
}

fn render_stats(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(ref stats) = app.stats {
        let stats_text = vec![
            Line::from(vec![
                Span::styled(" ↑ Uploaded:   ", Style::default().fg(Color::Green)),
                Span::styled(
                    format!("{:>12}", format_bytes(stats.uploaded)),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
                Span::raw("  @ "),
                Span::styled(
                    format!("{:>8.1} KB/s", stats.current_upload_rate),
                    Style::default().fg(Color::Green),
                ),
                Span::raw("  (avg: "),
                Span::styled(
                    format!("{:.1}", stats.average_upload_rate),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(")"),
            ]),
            Line::from(vec![
                Span::styled(" ↓ Downloaded: ", Style::default().fg(Color::Blue)),
                Span::styled(
                    format!("{:>12}", format_bytes(stats.downloaded)),
                    Style::default().fg(Color::White),
                ),
                Span::raw("  @ "),
                Span::styled(
                    format!("{:>8.1} KB/s", stats.current_download_rate),
                    Style::default().fg(Color::Blue),
                ),
                Span::raw("  (avg: "),
                Span::styled(
                    format!("{:.1}", stats.average_download_rate),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(")"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Ratio: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{:.3}", stats.ratio),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                Span::styled("Session: ", Style::default().fg(Color::Gray)),
                Span::styled(format!("{:.3}", stats.session_ratio), Style::default().fg(Color::Cyan)),
                Span::raw("   "),
                Span::styled("This session: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("↑{}", format_bytes(stats.session_uploaded)),
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Session time: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format_duration(stats.elapsed_time.as_secs()),
                    Style::default().fg(Color::White),
                ),
            ]),
        ];

        let stats_widget =
            Paragraph::new(stats_text).block(Block::default().borders(Borders::ALL).title(" Transfer Stats "));
        frame.render_widget(stats_widget, area);
    } else {
        let loading =
            Paragraph::new(" Loading stats...").block(Block::default().borders(Borders::ALL).title(" Transfer Stats "));
        frame.render_widget(loading, area);
    }
}

fn render_tracker_info(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![Span::styled(" Tracker: ", Style::default().fg(Color::Gray))];

    if let Some(ref stats) = app.stats {
        // Announce count
        spans.push(Span::styled(
            format!("{} announces", stats.announce_count),
            Style::default().fg(Color::White),
        ));

        // Last announce
        if let Some(last) = stats.last_announce {
            let ago = Instant::now().duration_since(last).as_secs();
            spans.push(Span::raw("   "));
            spans.push(Span::styled("Last: ", Style::default().fg(Color::Gray)));
            spans.push(Span::styled(
                format!("{}s ago", ago),
                Style::default().fg(Color::DarkGray),
            ));
        }

        // Next announce countdown
        if let Some(next) = stats.next_announce {
            let now = Instant::now();
            if next > now {
                let remaining = next.duration_since(now).as_secs();
                spans.push(Span::raw("   "));
                spans.push(Span::styled("Next: ", Style::default().fg(Color::Gray)));
                spans.push(Span::styled(
                    format_duration(remaining),
                    if remaining < 60 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Green)
                    },
                ));
            } else {
                spans.push(Span::raw("   "));
                spans.push(Span::styled("Next: ", Style::default().fg(Color::Gray)));
                spans.push(Span::styled("soon", Style::default().fg(Color::Yellow)));
            }
        }
    } else {
        spans.push(Span::styled("Waiting...", Style::default().fg(Color::DarkGray)));
    }

    let tracker_line = Line::from(spans);
    let tracker_info = Paragraph::new(tracker_line).block(Block::default().borders(Borders::ALL).title(" Announce "));
    frame.render_widget(tracker_info, area);
}

fn render_progress(frame: &mut Frame, app: &App, area: Rect) {
    // Split into multiple progress bars
    let mut constraints = Vec::new();
    let mut count = 0;

    if app.target_ratio.is_some() {
        constraints.push(Constraint::Length(1));
        count += 1;
    }
    if app.target_uploaded.is_some() {
        constraints.push(Constraint::Length(1));
        count += 1;
    }
    if app.target_time.is_some() {
        constraints.push(Constraint::Length(1));
        count += 1;
    }

    if count == 0 {
        return;
    }

    // Add spacing
    while constraints.len() < 3 {
        constraints.push(Constraint::Length(1));
    }

    let progress_block = Block::default().borders(Borders::ALL).title(" Progress ");
    let inner = progress_block.inner(area);
    frame.render_widget(progress_block, area);

    let progress_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let mut chunk_idx = 0;

    if let Some(ref stats) = app.stats {
        // Ratio progress
        if let Some(target) = app.target_ratio {
            let progress = (stats.ratio_progress).min(100.0);
            let eta_str = stats
                .eta_ratio
                .map(|d| format!(" ETA: {}", format_duration(d.as_secs())))
                .unwrap_or_default();

            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(Color::Cyan))
                .percent(progress as u16)
                .label(format!(
                    "Ratio: {:.2}/{:.1}x ({:.0}%){}",
                    stats.session_ratio, target, progress, eta_str
                ));
            frame.render_widget(gauge, progress_chunks[chunk_idx]);
            chunk_idx += 1;
        }

        // Upload progress
        if let Some(target_gb) = app.target_uploaded {
            let progress = (stats.upload_progress).min(100.0);
            let current_gb = stats.session_uploaded as f64 / (1024.0 * 1024.0 * 1024.0);
            let eta_str = stats
                .eta_uploaded
                .map(|d| format!(" ETA: {}", format_duration(d.as_secs())))
                .unwrap_or_default();

            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(Color::Green))
                .percent(progress as u16)
                .label(format!(
                    "Upload: {:.2}/{:.1} GB ({:.0}%){}",
                    current_gb, target_gb, progress, eta_str
                ));
            frame.render_widget(gauge, progress_chunks[chunk_idx]);
            chunk_idx += 1;
        }

        // Time progress
        if let Some(target_hours) = app.target_time {
            let progress = (stats.seed_time_progress).min(100.0);
            let current_hours = stats.elapsed_time.as_secs() as f64 / 3600.0;
            let eta_str = stats
                .eta_seed_time
                .map(|d| format!(" ETA: {}", format_duration(d.as_secs())))
                .unwrap_or_default();

            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(Color::Magenta))
                .percent(progress as u16)
                .label(format!(
                    "Time: {:.1}/{:.1}h ({:.0}%){}",
                    current_hours, target_hours, progress, eta_str
                ));
            frame.render_widget(gauge, progress_chunks[chunk_idx]);
        }
    }
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help = Paragraph::new(" [q] Quit   [p] Pause   [r] Resume   [x] Stop   [s] Scrape")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(help, area);
}
