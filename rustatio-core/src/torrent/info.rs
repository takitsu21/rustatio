use crate::protocol::bencode;
use crate::protocol::BencodeError;
use crate::{log_debug, log_error, log_trace};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fmt::Write;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TorrentError {
    #[error("Bencode error: {0}")]
    BencodeError(#[from] BencodeError),
    #[error("Invalid torrent structure: {0}")]
    InvalidStructure(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TorrentError>;

type BencodeDict = std::collections::HashMap<Vec<u8>, serde_bencode::value::Value>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TorrentInfo {
    /// SHA1 hash of the info dictionary (20 bytes)
    pub info_hash: [u8; 20],

    /// Announce URL (tracker)
    pub announce: String,

    /// Optional announce list for multiple trackers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub announce_list: Option<Vec<Vec<String>>>,

    /// Torrent name
    pub name: String,

    /// Total size in bytes
    pub total_size: u64,

    /// Piece length in bytes
    pub piece_length: u64,

    /// Number of pieces
    pub num_pieces: usize,

    /// Creation date (Unix timestamp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<i64>,

    /// Comment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    /// Created by
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    /// Is this a single-file or multi-file torrent
    pub is_single_file: bool,

    /// Number of files in the torrent
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub file_count: usize,

    /// File list (for multi-file torrents)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<TorrentFile>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TorrentSummary {
    /// SHA1 hash of the info dictionary (20 bytes)
    pub info_hash: [u8; 20],
    /// Announce URL (tracker)
    pub announce: String,
    /// Optional announce list for multiple trackers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub announce_list: Option<Vec<Vec<String>>>,
    /// Torrent name
    pub name: String,
    /// Total size in bytes
    pub total_size: u64,
    /// Piece length in bytes
    pub piece_length: u64,
    /// Number of pieces
    pub num_pieces: usize,
    /// Creation date (Unix timestamp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<i64>,
    /// Comment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Created by
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    /// Is this a single-file or multi-file torrent
    pub is_single_file: bool,
    /// Number of files (multi-file torrents)
    #[serde(default)]
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentFile {
    pub path: Vec<String>,
    pub length: u64,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}

impl TorrentInfo {
    /// Parse a torrent file from a path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        log_debug!("Loading torrent from file: {:?}", path.as_ref());
        let data = std::fs::read(path)?;
        Self::from_bytes(&data)
    }

    /// Parse a torrent from file without allocating file lists
    pub fn from_file_summary<P: AsRef<Path>>(path: P) -> Result<Self> {
        log_debug!("Loading torrent summary from file: {:?}", path.as_ref());
        let data = std::fs::read(path)?;
        Self::from_bytes_summary(&data)
    }

    /// Parse a torrent from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        log_trace!("Parsing torrent data ({} bytes)", data.len());

        let value = bencode::parse(data)?;

        let serde_bencode::value::Value::Dict(dict) = &value else {
            log_error!("Invalid torrent: root is not a dictionary");
            return Err(TorrentError::InvalidStructure("Root is not a dictionary".into()));
        };

        // Extract announce URL
        let announce = bencode::get_string(dict, "announce")?;

        // Extract announce-list (optional)
        let announce_list = dict
            .get(b"announce-list".as_ref())
            .and_then(|v| match v {
                serde_bencode::value::Value::List(list) => Some(list),
                _ => None,
            })
            .map(|list| {
                list.iter()
                    .filter_map(|tier| match tier {
                        serde_bencode::value::Value::List(t) => Some(t),
                        _ => None,
                    })
                    .map(|tier| {
                        tier.iter()
                            .filter_map(|url| match url {
                                serde_bencode::value::Value::Bytes(b) => {
                                    Some(String::from_utf8_lossy(b).to_string())
                                }
                                _ => None,
                            })
                            .collect()
                    })
                    .collect()
            });

        // Extract info dictionary
        let info_dict = dict
            .get(b"info".as_ref())
            .and_then(|v| match v {
                serde_bencode::value::Value::Dict(d) => Some(d),
                _ => None,
            })
            .ok_or_else(|| TorrentError::InvalidStructure("Missing info dictionary".into()))?;

        // Calculate info_hash (SHA1 of bencoded info dict)
        let info_hash = calculate_info_hash(data)?;

        // Extract name
        let name = bencode::get_string(info_dict, "name")?;

        // Extract piece length
        let piece_length = bencode::get_int(info_dict, "piece length")? as u64;

        // Extract pieces length only (avoid cloning piece hash data)
        let pieces_len = bencode::get_bytes_len(info_dict, "pieces")?;
        let num_pieces = pieces_len / 20;

        // Determine if single-file or multi-file
        let (is_single_file, total_size, files, file_count) = if let Ok(length) =
            bencode::get_int(info_dict, "length")
        {
            // Single file torrent
            (
                true,
                length as u64,
                vec![TorrentFile { path: vec![name.clone()], length: length as u64 }],
                1,
            )
        } else if let Some(files_list) = info_dict.get(b"files".as_ref()).and_then(|v| match v {
            serde_bencode::value::Value::List(l) => Some(l),
            _ => None,
        }) {
            // Multi-file torrent
            let mut files = Vec::new();
            let mut total = 0u64;
            let mut count = 0usize;

            for file_val in files_list {
                let serde_bencode::value::Value::Dict(file_dict) = file_val else {
                    return Err(TorrentError::InvalidStructure("Invalid file entry".into()));
                };

                let length = bencode::get_int(file_dict, "length")? as u64;

                let path = file_dict
                    .get(b"path".as_ref())
                    .and_then(|v| match v {
                        serde_bencode::value::Value::List(l) => Some(l),
                        _ => None,
                    })
                    .ok_or_else(|| TorrentError::InvalidStructure("Invalid file path".into()))?
                    .iter()
                    .filter_map(|p| match p {
                        serde_bencode::value::Value::Bytes(b) => {
                            Some(String::from_utf8_lossy(b).to_string())
                        }
                        _ => None,
                    })
                    .collect();

                files.push(TorrentFile { path, length });
                total += length;
                count += 1;
            }

            (false, total, files, count)
        } else {
            return Err(TorrentError::InvalidStructure(
                "Neither 'length' nor 'files' found in info dictionary".into(),
            ));
        };

        // Extract optional fields
        let creation_date = dict.get(b"creation date".as_ref()).and_then(|v| match v {
            serde_bencode::value::Value::Int(i) => Some(*i),
            _ => None,
        });
        let comment = dict.get(b"comment".as_ref()).and_then(|v| match v {
            serde_bencode::value::Value::Bytes(b) => Some(String::from_utf8_lossy(b).to_string()),
            _ => None,
        });
        let created_by = dict.get(b"created by".as_ref()).and_then(|v| match v {
            serde_bencode::value::Value::Bytes(b) => Some(String::from_utf8_lossy(b).to_string()),
            _ => None,
        });

        log_debug!(
            "Parsed torrent: name='{}', size={} bytes, pieces={}, tracker={}",
            name,
            total_size,
            num_pieces,
            announce
        );
        log_trace!(
            "Info hash: {}",
            info_hash.iter().fold(String::new(), |mut acc, b| {
                let _ = write!(acc, "{b:02x}");
                acc
            })
        );

        Ok(Self {
            info_hash,
            announce,
            announce_list,
            name,
            total_size,
            piece_length,
            num_pieces,
            creation_date,
            comment,
            created_by,
            is_single_file,
            file_count,
            files,
        })
    }

    /// Parse torrent data without allocating file lists
    pub fn from_bytes_summary(data: &[u8]) -> Result<Self> {
        let summary = TorrentSummary::from_bytes(data)?;
        Ok(summary.to_info())
    }

    /// Get the primary tracker URL
    pub fn get_tracker_url(&self) -> &str {
        &self.announce
    }

    /// Get all tracker URLs (from announce and announce-list)
    pub fn get_all_tracker_urls(&self) -> Vec<String> {
        let mut urls = vec![self.announce.clone()];

        if let Some(ref list) = self.announce_list {
            for tier in list {
                urls.extend(tier.iter().cloned());
            }
        }

        urls.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect()
    }

    /// Format `info_hash` as hex string (for debugging)
    pub fn info_hash_hex(&self) -> String {
        self.info_hash.iter().fold(String::new(), |mut acc, b| {
            let _ = write!(acc, "{b:02x}");
            acc
        })
    }

    /// Build a lightweight summary (excludes file list)
    pub fn summary(&self) -> TorrentSummary {
        let file_count = if self.file_count > 0 { self.file_count } else { self.files.len() };
        TorrentSummary {
            info_hash: self.info_hash,
            announce: self.announce.clone(),
            announce_list: self.announce_list.clone(),
            name: self.name.clone(),
            total_size: self.total_size,
            piece_length: self.piece_length,
            num_pieces: self.num_pieces,
            creation_date: self.creation_date,
            comment: self.comment.clone(),
            created_by: self.created_by.clone(),
            is_single_file: self.is_single_file,
            file_count,
        }
    }

    #[must_use]
    /// Drop file list data to reduce memory usage
    pub fn without_files(mut self) -> Self {
        self.files.clear();
        self.files.shrink_to_fit();
        self
    }
}

impl TorrentSummary {
    /// Parse a torrent summary from raw bytes (skips file list allocation)
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        log_trace!("Parsing torrent summary ({} bytes)", data.len());

        let value = bencode::parse(data)?;

        let dict = Self::root_dict(&value)?;
        let announce = bencode::get_string(dict, "announce")?;
        let announce_list = Self::announce_list(dict);
        let info_dict = Self::info_dict(dict)?;
        let info_hash = calculate_info_hash(data)?;
        let (name, piece_length, num_pieces) = Self::basic_info(info_dict)?;
        let (is_single_file, total_size, file_count) = Self::files_summary(info_dict)?;
        let (creation_date, comment, created_by) = Self::optional_fields(dict);

        Ok(Self {
            info_hash,
            announce,
            announce_list,
            name,
            total_size,
            piece_length,
            num_pieces,
            creation_date,
            comment,
            created_by,
            is_single_file,
            file_count,
        })
    }

    fn root_dict(value: &serde_bencode::value::Value) -> Result<&BencodeDict> {
        let serde_bencode::value::Value::Dict(dict) = value else {
            log_error!("Invalid torrent: root is not a dictionary");
            return Err(TorrentError::InvalidStructure("Root is not a dictionary".into()));
        };
        Ok(dict)
    }

    fn info_dict(dict: &BencodeDict) -> Result<&BencodeDict> {
        dict.get(b"info".as_ref())
            .and_then(|v| match v {
                serde_bencode::value::Value::Dict(d) => Some(d),
                _ => None,
            })
            .ok_or_else(|| TorrentError::InvalidStructure("Missing info dictionary".into()))
    }

    fn announce_list(dict: &BencodeDict) -> Option<Vec<Vec<String>>> {
        dict.get(b"announce-list".as_ref())
            .and_then(|v| match v {
                serde_bencode::value::Value::List(list) => Some(list),
                _ => None,
            })
            .map(|list| {
                list.iter()
                    .filter_map(|tier| match tier {
                        serde_bencode::value::Value::List(t) => Some(t),
                        _ => None,
                    })
                    .map(|tier| {
                        tier.iter()
                            .filter_map(|url| match url {
                                serde_bencode::value::Value::Bytes(b) => {
                                    Some(String::from_utf8_lossy(b).to_string())
                                }
                                _ => None,
                            })
                            .collect()
                    })
                    .collect()
            })
    }

    fn basic_info(info_dict: &BencodeDict) -> Result<(String, u64, usize)> {
        let name = bencode::get_string(info_dict, "name")?;
        let piece_length = bencode::get_int(info_dict, "piece length")? as u64;
        let pieces_len = bencode::get_bytes_len(info_dict, "pieces")?;
        let num_pieces = pieces_len / 20;
        Ok((name, piece_length, num_pieces))
    }

    fn files_summary(info_dict: &BencodeDict) -> Result<(bool, u64, usize)> {
        if let Ok(length) = bencode::get_int(info_dict, "length") {
            return Ok((true, length as u64, 1));
        }

        let Some(files_list) = info_dict.get(b"files".as_ref()).and_then(|v| match v {
            serde_bencode::value::Value::List(l) => Some(l),
            _ => None,
        }) else {
            return Err(TorrentError::InvalidStructure(
                "Neither 'length' nor 'files' found in info dictionary".into(),
            ));
        };

        let mut total = 0u64;
        let mut count = 0usize;

        for file_val in files_list {
            let serde_bencode::value::Value::Dict(file_dict) = file_val else {
                return Err(TorrentError::InvalidStructure("Invalid file entry".into()));
            };

            let length = bencode::get_int(file_dict, "length")? as u64;
            total += length;
            count += 1;
        }

        Ok((false, total, count))
    }

    fn optional_fields(dict: &BencodeDict) -> (Option<i64>, Option<String>, Option<String>) {
        let creation_date = dict.get(b"creation date".as_ref()).and_then(|v| match v {
            serde_bencode::value::Value::Int(i) => Some(*i),
            _ => None,
        });
        let comment = dict.get(b"comment".as_ref()).and_then(|v| match v {
            serde_bencode::value::Value::Bytes(b) => Some(String::from_utf8_lossy(b).to_string()),
            _ => None,
        });
        let created_by = dict.get(b"created by".as_ref()).and_then(|v| match v {
            serde_bencode::value::Value::Bytes(b) => Some(String::from_utf8_lossy(b).to_string()),
            _ => None,
        });

        (creation_date, comment, created_by)
    }

    /// Convert summary to a minimal `TorrentInfo` (empty file list)
    pub fn to_info(&self) -> TorrentInfo {
        TorrentInfo {
            info_hash: self.info_hash,
            announce: self.announce.clone(),
            announce_list: self.announce_list.clone(),
            name: self.name.clone(),
            total_size: self.total_size,
            piece_length: self.piece_length,
            num_pieces: self.num_pieces,
            creation_date: self.creation_date,
            comment: self.comment.clone(),
            created_by: self.created_by.clone(),
            is_single_file: self.is_single_file,
            file_count: self.file_count,
            files: Vec::new(),
        }
    }
}

/// Calculate the SHA1 `info_hash` from torrent bytes
fn calculate_info_hash(torrent_data: &[u8]) -> Result<[u8; 20]> {
    // Parse the torrent to find the info dictionary
    let value = bencode::parse(torrent_data)?;
    let serde_bencode::value::Value::Dict(_dict) = &value else {
        return Err(TorrentError::InvalidStructure("Root is not a dictionary".into()));
    };

    // We need to find the raw bytes of the info dictionary in the original data
    // This is a bit tricky because we need the exact bencoded representation

    // Find "4:info" in the data to locate the info dict
    let info_marker = b"4:info";
    let info_start = torrent_data
        .windows(info_marker.len())
        .position(|window| window == info_marker)
        .ok_or_else(|| TorrentError::InvalidStructure("Could not find info dictionary".into()))?
        + info_marker.len();

    // Parse just the info dictionary to get its bencoded representation
    let info_value =
        serde_bencode::from_bytes::<serde_bencode::value::Value>(&torrent_data[info_start..])
            .map_err(|e| BencodeError::ParseError(e.to_string()))?;

    let info_bytes = serde_bencode::to_bytes(&info_value)
        .map_err(|e| BencodeError::ParseError(e.to_string()))?;

    // Calculate SHA1
    let mut hasher = Sha1::new();
    hasher.update(&info_bytes);
    let result = hasher.finalize();

    let mut hash = [0u8; 20];
    hash.copy_from_slice(&result);
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_info_hash_hex() {
        let info = TorrentInfo {
            info_hash: [
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
                0xde, 0xf0, 0x12, 0x34, 0x56, 0x78,
            ],
            announce: "http://tracker.example.com/announce".to_string(),
            announce_list: None,
            name: "test".to_string(),
            total_size: 1024,
            piece_length: 256,
            num_pieces: 4,
            creation_date: None,
            comment: None,
            created_by: None,
            is_single_file: true,
            file_count: 1,
            files: vec![],
        };

        assert_eq!(info.info_hash_hex(), "123456789abcdef0123456789abcdef012345678");
    }
}
