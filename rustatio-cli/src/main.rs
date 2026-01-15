mod cli;
mod json;
mod runner;
mod session;
mod tui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use json::{format_bytes, ClientsOutput, TorrentInfoOutput};
use runner::RunnerConfig;
use session::Session;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger for non-JSON mode
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            torrent,
            client,
            client_version,
            upload_rate,
            download_rate,
            port,
            completion,
            initial_uploaded,
            initial_downloaded,
            stop_ratio,
            stop_uploaded,
            stop_downloaded,
            stop_time,
            stop_when_no_leechers,
            no_randomize,
            random_range,
            progressive,
            target_upload,
            target_download,
            progressive_duration,
            config: config_file,
            json,
            interval,
            resume,
            save_session,
            no_save_session,
        } => {
            // Validate torrent file exists
            if !torrent.exists() {
                if json {
                    json::OutputEvent::error(format!("Torrent file not found: {}", torrent.display())).emit();
                } else {
                    eprintln!("Error: Torrent file not found: {}", torrent.display());
                }
                std::process::exit(1);
            }

            // Load config file (if specified) or use defaults
            let app_config = load_config(config_file.as_ref(), json);

            // Load torrent to get info_hash for session lookup
            let torrent_info = runner::load_torrent(&torrent)?;
            let info_hash = torrent_info.info_hash_hex();

            // Try to load existing session if --resume is set
            let existing_session = if resume {
                Session::load_for_hash(&info_hash)
            } else {
                None
            };

            // Determine initial values: session > CLI args > config defaults
            let (effective_uploaded, effective_downloaded) = if let Some(ref session) = existing_session {
                if !json {
                    eprintln!(
                        "Resuming session: {} uploaded, ratio {:.3}",
                        format_bytes(session.uploaded),
                        session.ratio()
                    );
                }
                (session.uploaded, session.downloaded)
            } else {
                (initial_uploaded, initial_downloaded)
            };

            // Apply config defaults where CLI args use defaults
            let effective_upload_rate = if upload_rate == 50.0 {
                app_config.faker.default_upload_rate
            } else {
                upload_rate
            };

            let effective_download_rate = if download_rate == 100.0 {
                app_config.faker.default_download_rate
            } else {
                download_rate
            };

            let effective_port = if port == 6881 {
                app_config.client.default_port
            } else {
                port
            };

            let config = RunnerConfig {
                torrent_path: torrent,
                client,
                client_version: client_version.or(app_config.client.default_version.clone()),
                upload_rate: effective_upload_rate,
                download_rate: effective_download_rate,
                port: effective_port,
                completion,
                initial_uploaded: effective_uploaded,
                initial_downloaded: effective_downloaded,
                stop_ratio,
                stop_uploaded,
                stop_downloaded,
                stop_time,
                stop_when_no_leechers,
                no_randomize,
                random_range,
                progressive,
                target_upload,
                target_download,
                progressive_duration,
                json_mode: json,
                stats_interval: interval,
                save_session: save_session && !no_save_session,
                info_hash: info_hash.clone(),
                torrent_name: torrent_info.name.clone(),
                torrent_size: torrent_info.total_size,
            };

            if json {
                runner::run_json_mode(config).await?;
            } else {
                tui::run_tui_mode(config).await?;
            }
        }

        Commands::Resume {
            info_hash,
            upload_rate,
            download_rate,
            stop_ratio,
            stop_uploaded,
            json,
            interval,
            no_save_session,
        } => {
            // Look up the session
            let session = match Session::load_for_hash(&info_hash) {
                Some(s) => s,
                None => {
                    if json {
                        json::OutputEvent::error(format!("Session not found: {}", info_hash)).emit();
                    } else {
                        eprintln!("Error: No saved session found for hash: {}", info_hash);
                        eprintln!();
                        eprintln!("Run `rustatio sessions` to list available sessions.");
                    }
                    std::process::exit(1);
                }
            };

            // Check if torrent file still exists
            let torrent_path = std::path::PathBuf::from(&session.torrent_path);
            if !torrent_path.exists() {
                if json {
                    json::OutputEvent::error(format!("Torrent file no longer exists: {}", session.torrent_path)).emit();
                } else {
                    eprintln!("Error: Torrent file no longer exists: {}", session.torrent_path);
                    eprintln!();
                    eprintln!("The session was saved but the torrent file has been moved or deleted.");
                }
                std::process::exit(1);
            }

            if !json {
                eprintln!(
                    "Resuming session: {} uploaded, ratio {}",
                    format_bytes(session.uploaded),
                    if session.ratio().is_infinite() {
                        "inf".to_string()
                    } else {
                        format!("{:.3}", session.ratio())
                    }
                );
            }

            // Parse client type from session
            let client = match session.client.to_lowercase().as_str() {
                "qbittorrent" => cli::ClientArg::Qbittorrent,
                "utorrent" => cli::ClientArg::Utorrent,
                "transmission" => cli::ClientArg::Transmission,
                "deluge" => cli::ClientArg::Deluge,
                _ => cli::ClientArg::Qbittorrent,
            };

            let config = RunnerConfig {
                torrent_path,
                client,
                client_version: session.client_version.clone(),
                upload_rate: upload_rate.unwrap_or(session.upload_rate),
                download_rate: download_rate.unwrap_or(session.download_rate),
                port: session.port,
                completion: session.completion_percent,
                initial_uploaded: session.uploaded,
                initial_downloaded: session.downloaded,
                stop_ratio: stop_ratio.or(session.stop_at_ratio),
                stop_uploaded: stop_uploaded.or(session.stop_at_uploaded_gb),
                stop_downloaded: None,
                stop_time: None,
                stop_when_no_leechers: false,
                no_randomize: false,
                random_range: 20.0,
                progressive: false,
                target_upload: None,
                target_download: None,
                progressive_duration: 1.0,
                json_mode: json,
                stats_interval: interval,
                save_session: !no_save_session,
                info_hash: session.info_hash.clone(),
                torrent_name: session.torrent_name.clone(),
                torrent_size: session.torrent_size,
            };

            if json {
                runner::run_json_mode(config).await?;
            } else {
                tui::run_tui_mode(config).await?;
            }
        }

        Commands::Info { torrent, json } => {
            if !torrent.exists() {
                if json {
                    json::OutputEvent::error(format!("Torrent file not found: {}", torrent.display())).emit();
                } else {
                    eprintln!("Error: Torrent file not found: {}", torrent.display());
                }
                std::process::exit(1);
            }

            let torrent_info = runner::load_torrent(&torrent)?;

            if json {
                let output = TorrentInfoOutput::from(&torrent_info);
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                print_torrent_info(&torrent_info);
            }
        }

        Commands::Clients { json } => {
            let clients = ClientsOutput::new();

            if json {
                println!("{}", serde_json::to_string_pretty(&clients)?);
            } else {
                println!("Supported BitTorrent Clients:");
                println!();
                for client in &clients.clients {
                    println!(
                        "  {:14} {} (default: {})",
                        client.id, client.name, client.default_version
                    );
                }
                println!();
                println!("Use --client <id> to select a client.");
            }
        }

        Commands::Config {
            init,
            path,
            show,
            json: json_output,
        } => {
            let config_path = rustatio_core::AppConfig::default_path();

            if path {
                if json_output {
                    println!("{}", serde_json::json!({ "path": config_path.display().to_string() }));
                } else {
                    println!("{}", config_path.display());
                }
            } else if init {
                if config_path.exists() {
                    if json_output {
                        json::OutputEvent::error("Config file already exists").emit();
                    } else {
                        eprintln!("Config file already exists at: {}", config_path.display());
                        eprintln!("Use --show to view current config.");
                    }
                    std::process::exit(1);
                }

                // Create parent directory
                if let Some(parent) = config_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                // Write default config
                let config = rustatio_core::AppConfig::default();
                config.save(&config_path)?;

                if json_output {
                    println!(
                        "{}",
                        serde_json::json!({
                            "created": true,
                            "path": config_path.display().to_string()
                        })
                    );
                } else {
                    println!("Created default config at: {}", config_path.display());
                }
            } else if show {
                if !config_path.exists() {
                    if json_output {
                        json::OutputEvent::error("No config file found").emit();
                    } else {
                        eprintln!("No config file found at: {}", config_path.display());
                        eprintln!("Use --init to create one.");
                    }
                    std::process::exit(1);
                }

                let content = std::fs::read_to_string(&config_path)?;

                if json_output {
                    let config = rustatio_core::AppConfig::load(&config_path)?;
                    println!("{}", serde_json::to_string_pretty(&config)?);
                } else {
                    println!("# Config file: {}", config_path.display());
                    println!();
                    println!("{}", content);
                }
            } else {
                // Show help if no subcommand
                println!("Config management commands:");
                println!();
                println!("  rustatio config --path   Show config file path");
                println!("  rustatio config --init   Create default config file");
                println!("  rustatio config --show   Show current configuration");
            }
        }

        Commands::Sessions {
            delete,
            clear,
            path,
            json: json_output,
        } => {
            if path {
                let sessions_dir = Session::sessions_dir();
                if json_output {
                    println!("{}", serde_json::json!({ "path": sessions_dir.display().to_string() }));
                } else {
                    println!("{}", sessions_dir.display());
                }
            } else if clear {
                let sessions = Session::list_all()?;
                let count = sessions.len();

                for summary in sessions {
                    if let Some(session) = Session::load_for_hash(&summary.info_hash) {
                        let _ = session.delete();
                    }
                }

                if json_output {
                    println!("{}", serde_json::json!({ "deleted": count }));
                } else {
                    println!("Deleted {} session(s)", count);
                }
            } else if let Some(hash) = delete {
                if let Some(session) = Session::load_for_hash(&hash) {
                    session.delete()?;
                    if json_output {
                        println!("{}", serde_json::json!({ "deleted": true, "info_hash": hash }));
                    } else {
                        println!("Deleted session for {}", hash);
                    }
                } else {
                    if json_output {
                        json::OutputEvent::error(format!("Session not found: {}", hash)).emit();
                    } else {
                        eprintln!("Session not found: {}", hash);
                    }
                    std::process::exit(1);
                }
            } else {
                // List all sessions
                let sessions = Session::list_all()?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&sessions)?);
                } else if sessions.is_empty() {
                    println!("No saved sessions found.");
                    println!();
                    println!("Sessions are created when you run `rustatio start` (saved by default).");
                    println!("Use --resume to continue from a saved session.");
                } else {
                    use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};

                    let mut table = Table::new();
                    table
                        .load_preset(UTF8_FULL_CONDENSED)
                        .set_content_arrangement(ContentArrangement::Dynamic)
                        .set_header(vec![
                            Cell::new("#").fg(Color::DarkGrey),
                            Cell::new("Torrent").fg(Color::Cyan),
                            Cell::new("Uploaded").fg(Color::Green),
                            Cell::new("Ratio").fg(Color::Yellow),
                            Cell::new("Time").fg(Color::Blue),
                            Cell::new("Last Active").fg(Color::Magenta),
                        ]);

                    for (i, session) in sessions.iter().enumerate() {
                        // Truncate name if too long
                        let name = if session.torrent_name.len() > 45 {
                            format!("{}…", &session.torrent_name[..44])
                        } else {
                            session.torrent_name.clone()
                        };

                        // Format seed time
                        let seed_time = json::format_duration(session.total_seed_time_secs);

                        // Format last active (relative time)
                        let last_active = format_relative_time(session.updated_at);

                        table.add_row(vec![
                            Cell::new(i + 1).fg(Color::DarkGrey),
                            Cell::new(name),
                            Cell::new(format_bytes(session.uploaded)).fg(Color::Green),
                            Cell::new(&session.ratio_display).fg(Color::Yellow),
                            Cell::new(seed_time).fg(Color::Blue),
                            Cell::new(last_active).fg(Color::Magenta),
                        ]);
                    }

                    println!("{table}");
                    println!();

                    // Show info hashes in a separate section for easy copying
                    println!("Session Hashes (for resume/delete):");
                    for (i, session) in sessions.iter().enumerate() {
                        println!("  {} → {}", i + 1, session.info_hash);
                    }
                    println!();
                    println!("Commands:");
                    println!("  rustatio resume <hash>            Resume a session by hash");
                    println!("  rustatio start <file> --resume    Resume by torrent file");
                    println!("  rustatio sessions --delete <hash> Delete a session");
                }
            }
        }

        Commands::Completions { shell } => {
            Cli::generate_completions(shell.into());
        }
    }

    Ok(())
}

/// Load configuration from file or use defaults
fn load_config(config_path: Option<&std::path::PathBuf>, json_mode: bool) -> rustatio_core::AppConfig {
    if let Some(path) = config_path {
        match rustatio_core::AppConfig::load(path) {
            Ok(config) => config,
            Err(e) => {
                if json_mode {
                    json::OutputEvent::error(format!("Failed to load config: {}", e)).emit();
                } else {
                    eprintln!("Warning: Failed to load config from {:?}: {}", path, e);
                    eprintln!("Using default configuration.");
                }
                rustatio_core::AppConfig::default()
            }
        }
    } else {
        // Try to load from default path, but don't fail if not found
        rustatio_core::AppConfig::load_or_default()
    }
}

fn print_torrent_info(torrent: &rustatio_core::TorrentInfo) {
    println!("Torrent Information");
    println!("===================");
    println!();
    println!("Name:        {}", torrent.name);
    println!("Size:        {}", format_bytes(torrent.total_size));
    println!("Info Hash:   {}", torrent.info_hash_hex());
    println!();
    println!("Tracker:     {}", torrent.announce);

    if let Some(ref list) = torrent.announce_list {
        if !list.is_empty() {
            println!("Announce List:");
            for (i, tier) in list.iter().enumerate() {
                for url in tier {
                    println!("  Tier {}: {}", i + 1, url);
                }
            }
        }
    }

    println!();
    println!(
        "Pieces:      {} x {}",
        torrent.num_pieces,
        format_bytes(torrent.piece_length)
    );

    if let Some(date) = torrent.creation_date {
        if let Some(dt) = chrono::DateTime::from_timestamp(date, 0) {
            println!("Created:     {}", dt.format("%Y-%m-%d %H:%M:%S UTC"));
        }
    }

    if let Some(ref by) = torrent.created_by {
        println!("Created By:  {}", by);
    }

    if let Some(ref comment) = torrent.comment {
        println!("Comment:     {}", comment);
    }

    println!();

    if torrent.is_single_file {
        println!("Type:        Single file");
    } else {
        println!("Type:        Multi-file ({} files)", torrent.files.len());
        println!();
        println!("Files:");
        for file in &torrent.files {
            let path = file.path.join("/");
            println!("  {:>12}  {}", format_bytes(file.length), path);
        }
    }
}

/// Format a datetime as relative time (e.g., "2h ago", "3d ago")
fn format_relative_time(dt: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(dt);

    if duration.num_seconds() < 60 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{}m ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h ago", duration.num_hours())
    } else if duration.num_days() < 30 {
        format!("{}d ago", duration.num_days())
    } else if duration.num_days() < 365 {
        format!("{}mo ago", duration.num_days() / 30)
    } else {
        format!("{}y ago", duration.num_days() / 365)
    }
}
