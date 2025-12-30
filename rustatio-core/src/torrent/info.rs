use crate::protocol::bencode;
use crate::protocol::BencodeError;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// File list (for multi-file torrents)
    pub files: Vec<TorrentFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentFile {
    pub path: Vec<String>,
    pub length: u64,
}

impl TorrentInfo {
    /// Parse a torrent file from a path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data = std::fs::read(path)?;
        Self::from_bytes(&data)
    }

    /// Parse a torrent from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let value = bencode::parse(data)?;

        let dict = match &value {
            serde_bencode::value::Value::Dict(d) => d,
            _ => return Err(TorrentError::InvalidStructure("Root is not a dictionary".into())),
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
                                serde_bencode::value::Value::Bytes(b) => Some(String::from_utf8_lossy(b).to_string()),
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

        // Extract pieces
        let pieces_bytes = bencode::get_bytes(info_dict, "pieces")?;
        let num_pieces = pieces_bytes.len() / 20;

        // Determine if single-file or multi-file
        let (is_single_file, total_size, files) = if let Ok(length) = bencode::get_int(info_dict, "length") {
            // Single file torrent
            (
                true,
                length as u64,
                vec![TorrentFile {
                    path: vec![name.clone()],
                    length: length as u64,
                }],
            )
        } else if let Some(files_list) = info_dict.get(b"files".as_ref()).and_then(|v| match v {
            serde_bencode::value::Value::List(l) => Some(l),
            _ => None,
        }) {
            // Multi-file torrent
            let mut files = Vec::new();
            let mut total = 0u64;

            for file_val in files_list {
                let file_dict = match file_val {
                    serde_bencode::value::Value::Dict(d) => d,
                    _ => return Err(TorrentError::InvalidStructure("Invalid file entry".into())),
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
                        serde_bencode::value::Value::Bytes(b) => Some(String::from_utf8_lossy(b).to_string()),
                        _ => None,
                    })
                    .collect();

                files.push(TorrentFile { path, length });
                total += length;
            }

            (false, total, files)
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

        Ok(TorrentInfo {
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
            files,
        })
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

        urls.into_iter()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// Format info_hash as hex string (for debugging)
    pub fn info_hash_hex(&self) -> String {
        self.info_hash.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// Calculate the SHA1 info_hash from torrent bytes
fn calculate_info_hash(torrent_data: &[u8]) -> Result<[u8; 20]> {
    // Parse the torrent to find the info dictionary
    let value = bencode::parse(torrent_data)?;
    let _dict = match &value {
        serde_bencode::value::Value::Dict(d) => d,
        _ => return Err(TorrentError::InvalidStructure("Root is not a dictionary".into())),
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
    let info_value = serde_bencode::from_bytes::<serde_bencode::value::Value>(&torrent_data[info_start..])
        .map_err(|e| BencodeError::ParseError(e.to_string()))?;

    let info_bytes = serde_bencode::to_bytes(&info_value).map_err(|e| BencodeError::ParseError(e.to_string()))?;

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
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12,
                0x34, 0x56, 0x78,
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
            files: vec![],
        };

        assert_eq!(info.info_hash_hex(), "123456789abcdef0123456789abcdef012345678");
    }
}
