use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use std::io;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "rustatio")]
#[command(author, version, about = "BitTorrent ratio faker CLI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    /// Generate shell completions and print to stdout
    pub fn generate_completions(shell: Shell) {
        let mut cmd = Cli::command();
        generate(shell, &mut cmd, "rustatio", &mut io::stdout());
    }
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)] // Start command has many options by design
pub enum Commands {
    /// Start faking ratio for a torrent
    Start {
        /// Path to the .torrent file
        #[arg(value_name = "TORRENT_FILE")]
        torrent: PathBuf,

        /// Client to emulate
        #[arg(short, long, value_enum, default_value = "qbittorrent")]
        client: ClientArg,

        /// Client version string (e.g., "5.1.4")
        #[arg(long, value_name = "VERSION")]
        client_version: Option<String>,

        /// Upload rate in KB/s
        #[arg(short, long, default_value = "50.0", value_name = "KB/s")]
        upload_rate: f64,

        /// Download rate in KB/s
        #[arg(short, long, default_value = "100.0", value_name = "KB/s")]
        download_rate: f64,

        /// Port to announce
        #[arg(short, long, default_value = "6881")]
        port: u16,

        /// Initial completion percentage (0-100)
        #[arg(long, default_value = "0.0", value_name = "PERCENT")]
        completion: f64,

        /// Initial uploaded bytes (for continuing sessions)
        #[arg(long, default_value = "0", value_name = "BYTES")]
        initial_uploaded: u64,

        /// Initial downloaded bytes (for continuing sessions)
        #[arg(long, default_value = "0", value_name = "BYTES")]
        initial_downloaded: u64,

        /// Stop when session ratio reaches this value
        #[arg(long, value_name = "RATIO")]
        stop_ratio: Option<f64>,

        /// Stop after uploading this many gigabytes
        #[arg(long, value_name = "GB")]
        stop_uploaded: Option<f64>,

        /// Stop after downloading this many gigabytes
        #[arg(long, value_name = "GB")]
        stop_downloaded: Option<f64>,

        /// Stop after running for this many hours
        #[arg(long, value_name = "HOURS")]
        stop_time: Option<f64>,

        /// Stop when there are no leechers
        #[arg(long)]
        stop_when_no_leechers: bool,

        /// Disable rate randomization
        #[arg(long)]
        no_randomize: bool,

        /// Randomization range percentage (default: 20%)
        #[arg(long, default_value = "20.0", value_name = "PERCENT")]
        random_range: f64,

        /// Enable progressive rate adjustment
        #[arg(long)]
        progressive: bool,

        /// Target upload rate for progressive mode (KB/s)
        #[arg(long, value_name = "KB/s")]
        target_upload: Option<f64>,

        /// Target download rate for progressive mode (KB/s)
        #[arg(long, value_name = "KB/s")]
        target_download: Option<f64>,

        /// Duration to reach target rates (hours)
        #[arg(long, default_value = "1.0", value_name = "HOURS")]
        progressive_duration: f64,

        /// Path to config file
        #[arg(long, value_name = "FILE")]
        config: Option<PathBuf>,

        /// Output JSON Lines instead of TUI (for integrations)
        #[arg(long)]
        json: bool,

        /// Stats update interval in seconds (JSON mode only)
        #[arg(long, default_value = "1", value_name = "SECONDS")]
        interval: u64,

        /// Resume from saved session (if exists for this torrent)
        #[arg(long)]
        resume: bool,

        /// Save session progress on exit (enabled by default)
        #[arg(long, default_value = "true")]
        save_session: bool,

        /// Don't save session progress on exit
        #[arg(long)]
        no_save_session: bool,
    },

    /// Resume a saved session by info hash
    Resume {
        /// Info hash of the session to resume (from `rustatio sessions`)
        #[arg(value_name = "INFO_HASH")]
        info_hash: String,

        /// Override upload rate (KB/s)
        #[arg(short, long, value_name = "KB/s")]
        upload_rate: Option<f64>,

        /// Override download rate (KB/s)
        #[arg(short, long, value_name = "KB/s")]
        download_rate: Option<f64>,

        /// Stop when session ratio reaches this value
        #[arg(long, value_name = "RATIO")]
        stop_ratio: Option<f64>,

        /// Stop after uploading this many gigabytes
        #[arg(long, value_name = "GB")]
        stop_uploaded: Option<f64>,

        /// Output JSON Lines instead of TUI
        #[arg(long)]
        json: bool,

        /// Stats update interval in seconds (JSON mode only)
        #[arg(long, default_value = "1", value_name = "SECONDS")]
        interval: u64,

        /// Don't save session progress on exit
        #[arg(long)]
        no_save_session: bool,
    },

    /// Display information about a torrent file
    Info {
        /// Path to the .torrent file
        #[arg(value_name = "TORRENT_FILE")]
        torrent: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List supported BitTorrent clients
    Clients {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Manage configuration
    Config {
        /// Create a default configuration file
        #[arg(long)]
        init: bool,

        /// Show config file path
        #[arg(long)]
        path: bool,

        /// Show current configuration
        #[arg(long)]
        show: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List and manage saved sessions
    Sessions {
        /// Delete a session by info hash
        #[arg(long, value_name = "INFO_HASH")]
        delete: Option<String>,

        /// Clear all sessions
        #[arg(long)]
        clear: bool,

        /// Show sessions directory path
        #[arg(long)]
        path: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: ShellArg,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ClientArg {
    Qbittorrent,
    Utorrent,
    Transmission,
    Deluge,
}

impl From<ClientArg> for rustatio_core::ClientType {
    fn from(client: ClientArg) -> Self {
        match client {
            ClientArg::Qbittorrent => rustatio_core::ClientType::QBittorrent,
            ClientArg::Utorrent => rustatio_core::ClientType::UTorrent,
            ClientArg::Transmission => rustatio_core::ClientType::Transmission,
            ClientArg::Deluge => rustatio_core::ClientType::Deluge,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ShellArg {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

impl From<ShellArg> for Shell {
    fn from(shell: ShellArg) -> Self {
        match shell {
            ShellArg::Bash => Shell::Bash,
            ShellArg::Zsh => Shell::Zsh,
            ShellArg::Fish => Shell::Fish,
            ShellArg::PowerShell => Shell::PowerShell,
            ShellArg::Elvish => Shell::Elvish,
        }
    }
}
