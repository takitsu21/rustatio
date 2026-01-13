//! Session persistence for rustatio-cli
//!
//! Sessions allow users to save and restore faking progress across restarts.
//! Session files are stored as JSON in the sessions directory.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Session data that persists across restarts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session format version
    pub version: u32,

    /// Info hash of the torrent (hex string)
    pub info_hash: String,

    /// Torrent name for display
    pub torrent_name: String,

    /// Path to the torrent file (for resuming)
    pub torrent_path: String,

    /// Torrent total size in bytes
    #[serde(default)]
    pub torrent_size: u64,

    /// Client type used
    pub client: String,

    /// Client version
    pub client_version: Option<String>,

    /// Total uploaded bytes (cumulative across all sessions)
    pub uploaded: u64,

    /// Total downloaded bytes (cumulative)
    pub downloaded: u64,

    /// Upload rate (KB/s)
    pub upload_rate: f64,

    /// Download rate (KB/s)
    pub download_rate: f64,

    /// Port used
    pub port: u16,

    /// Completion percentage
    pub completion_percent: f64,

    /// Total time spent faking (seconds)
    pub total_seed_time_secs: u64,

    /// When the session was created
    pub created_at: DateTime<Utc>,

    /// When the session was last updated
    pub updated_at: DateTime<Utc>,

    /// Target ratio (if set)
    pub stop_at_ratio: Option<f64>,

    /// Target uploaded GB (if set)
    pub stop_at_uploaded_gb: Option<f64>,
}

impl Session {
    /// Current session format version
    pub const VERSION: u32 = 1;

    /// Create a new session
    pub fn new(
        info_hash: &str,
        torrent_name: &str,
        torrent_path: &str,
        torrent_size: u64,
        client: &str,
        client_version: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Session {
            version: Self::VERSION,
            info_hash: info_hash.to_string(),
            torrent_name: torrent_name.to_string(),
            torrent_path: torrent_path.to_string(),
            torrent_size,
            client: client.to_string(),
            client_version,
            uploaded: 0,
            downloaded: 0,
            upload_rate: 50.0,
            download_rate: 100.0,
            port: 6881,
            completion_percent: 0.0,
            total_seed_time_secs: 0,
            created_at: now,
            updated_at: now,
            stop_at_ratio: None,
            stop_at_uploaded_gb: None,
        }
    }

    /// Update session with current stats
    pub fn update(&mut self, uploaded: u64, downloaded: u64, elapsed_secs: u64) {
        self.uploaded = uploaded;
        self.downloaded = downloaded;
        self.total_seed_time_secs += elapsed_secs;
        self.updated_at = Utc::now();
    }

    /// Calculate current ratio (uploaded / torrent_size)
    /// This represents how many times you've "uploaded" the torrent
    pub fn ratio(&self) -> f64 {
        if self.torrent_size > 0 {
            self.uploaded as f64 / self.torrent_size as f64
        } else if self.uploaded > 0 {
            f64::INFINITY
        } else {
            0.0
        }
    }

    /// Load a session from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read session file: {:?}", path.as_ref()))?;
        let mut session: Session = serde_json::from_str(&content).with_context(|| "Failed to parse session file")?;

        // Migrate old sessions: try to get torrent_size from the torrent file
        if session.torrent_size == 0 {
            if let Ok(torrent) = rustatio_core::TorrentInfo::from_file(&session.torrent_path) {
                session.torrent_size = torrent.total_size;
            }
        }

        Ok(session)
    }

    /// Save session to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent).with_context(|| format!("Failed to create session directory: {:?}", parent))?;
        }

        let content = serde_json::to_string_pretty(self).with_context(|| "Failed to serialize session")?;
        fs::write(path.as_ref(), content)
            .with_context(|| format!("Failed to write session file: {:?}", path.as_ref()))?;
        Ok(())
    }

    /// Get the default sessions directory
    pub fn sessions_dir() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config").join("rustatio").join("sessions")
        } else {
            PathBuf::from("sessions")
        }
    }

    /// Get the session file path for an info hash
    pub fn path_for_hash(info_hash: &str) -> PathBuf {
        Self::sessions_dir().join(format!("{}.json", info_hash))
    }

    /// Load session by info hash (if exists)
    pub fn load_for_hash(info_hash: &str) -> Option<Self> {
        let path = Self::path_for_hash(info_hash);
        if path.exists() {
            Self::load(&path).ok()
        } else {
            None
        }
    }

    /// Save session (uses info_hash as filename)
    pub fn save_session(&self) -> Result<()> {
        let path = Self::path_for_hash(&self.info_hash);
        self.save(path)
    }

    /// Delete session file
    pub fn delete(&self) -> Result<()> {
        let path = Self::path_for_hash(&self.info_hash);
        if path.exists() {
            fs::remove_file(&path).with_context(|| format!("Failed to delete session file: {:?}", path))?;
        }
        Ok(())
    }

    /// List all saved sessions
    pub fn list_all() -> Result<Vec<SessionSummary>> {
        let dir = Self::sessions_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }

        let mut sessions = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(session) = Self::load(&path) {
                    sessions.push(SessionSummary::from(&session));
                }
            }
        }

        // Sort by last updated (most recent first)
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }
}

/// Summary information about a session for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub info_hash: String,
    pub torrent_name: String,
    pub uploaded: u64,
    pub downloaded: u64,
    /// Ratio (None if infinite/undefined)
    #[serde(skip_serializing_if = "is_infinite_ratio")]
    pub ratio: Option<f64>,
    /// Display ratio as string (handles inf)
    pub ratio_display: String,
    pub total_seed_time_secs: u64,
    pub updated_at: DateTime<Utc>,
}

fn is_infinite_ratio(r: &Option<f64>) -> bool {
    r.map(|v| v.is_infinite()).unwrap_or(true)
}

impl From<&Session> for SessionSummary {
    fn from(session: &Session) -> Self {
        let ratio = session.ratio();
        SessionSummary {
            info_hash: session.info_hash.clone(),
            torrent_name: session.torrent_name.clone(),
            uploaded: session.uploaded,
            downloaded: session.downloaded,
            ratio: if ratio.is_infinite() { None } else { Some(ratio) },
            ratio_display: if ratio.is_infinite() {
                "inf".to_string()
            } else {
                format!("{:.3}", ratio)
            },
            total_seed_time_secs: session.total_seed_time_secs,
            updated_at: session.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_session_create_and_update() {
        // torrent_size = 50 MB, so 100 MB uploaded = ratio 2.0
        let torrent_size = 1024 * 1024 * 50; // 50 MB
        let mut session = Session::new(
            "abcdef1234567890",
            "Test Torrent",
            "/path/to/test.torrent",
            torrent_size,
            "qbittorrent",
            Some("5.1.4".to_string()),
        );

        assert_eq!(session.uploaded, 0);
        assert_eq!(session.ratio(), 0.0);

        session.update(1024 * 1024 * 100, 1024 * 1024 * 50, 3600);
        assert_eq!(session.uploaded, 1024 * 1024 * 100);
        // 100 MB uploaded / 50 MB torrent size = 2.0 ratio
        assert_eq!(session.ratio(), 2.0);
    }

    #[test]
    fn test_session_save_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_session.json");

        let session = Session::new(
            "abcdef1234567890",
            "Test Torrent",
            "/path/to/test.torrent",
            1024 * 1024 * 100, // 100 MB
            "qbittorrent",
            Some("5.1.4".to_string()),
        );

        session.save(&path).unwrap();
        assert!(path.exists());

        let loaded = Session::load(&path).unwrap();
        assert_eq!(loaded.info_hash, session.info_hash);
        assert_eq!(loaded.torrent_name, session.torrent_name);
        assert_eq!(loaded.torrent_size, session.torrent_size);
    }
}
