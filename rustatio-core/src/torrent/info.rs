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
    use serde_bencode::value::Value;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn info_hash(data: &[u8]) -> Result<[u8; 20]> {
        calculate_info_hash(data)
    }

    fn dict(entries: Vec<(Vec<u8>, Value)>) -> Value {
        let mut map = HashMap::new();
        for (key, value) in entries {
            map.insert(key, value);
        }
        Value::Dict(map)
    }

    fn bytes(value: &str) -> Value {
        Value::Bytes(value.as_bytes().to_vec())
    }

    fn int(value: i64) -> Value {
        Value::Int(value)
    }

    fn list(values: Vec<Value>) -> Value {
        Value::List(values)
    }

    fn pieces(count: usize) -> Value {
        let data = vec![0u8; count * 20];
        Value::Bytes(data)
    }

    fn sample_single_file() -> Value {
        dict(vec![
            (b"announce".to_vec(), bytes("http://tracker.test/announce")),
            (
                b"info".to_vec(),
                dict(vec![
                    (b"name".to_vec(), bytes("file.txt")),
                    (b"piece length".to_vec(), int(16384)),
                    (b"pieces".to_vec(), pieces(2)),
                    (b"length".to_vec(), int(123)),
                ]),
            ),
        ])
    }

    fn sample_multi_file() -> Value {
        let file_a = dict(vec![
            (b"length".to_vec(), int(100)),
            (b"path".to_vec(), list(vec![bytes("dir"), bytes("a.bin")])),
        ]);
        let file_b = dict(vec![
            (b"length".to_vec(), int(50)),
            (b"path".to_vec(), list(vec![bytes("dir"), bytes("b.bin")])),
        ]);

        dict(vec![
            (b"announce".to_vec(), bytes("http://tracker.test/announce")),
            (
                b"announce-list".to_vec(),
                list(vec![list(vec![bytes("http://tracker.test/announce")])]),
            ),
            (
                b"info".to_vec(),
                dict(vec![
                    (b"name".to_vec(), bytes("folder")),
                    (b"piece length".to_vec(), int(16384)),
                    (b"pieces".to_vec(), pieces(1)),
                    (b"files".to_vec(), list(vec![file_a, file_b])),
                ]),
            ),
        ])
    }

    fn encode(value: &Value) -> Result<Vec<u8>> {
        bencode::encode(value).map_err(TorrentError::from)
    }

    fn temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(name);
        path
    }

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

    #[test]
    fn test_from_bytes_single_file() -> Result<()> {
        let data = encode(&sample_single_file())?;
        let torrent = TorrentInfo::from_bytes(&data)?;

        assert_eq!(torrent.announce, "http://tracker.test/announce");
        assert_eq!(torrent.name, "file.txt");
        assert_eq!(torrent.total_size, 123);
        assert_eq!(torrent.piece_length, 16384);
        assert_eq!(torrent.num_pieces, 2);
        assert!(torrent.is_single_file);
        assert_eq!(torrent.file_count, 1);
        assert_eq!(torrent.files.len(), 1);
        Ok(())
    }

    #[test]
    fn test_from_bytes_multi_file() -> Result<()> {
        let data = encode(&sample_multi_file())?;
        let torrent = TorrentInfo::from_bytes(&data)?;

        assert_eq!(torrent.total_size, 150);
        assert_eq!(torrent.file_count, 2);
        assert!(!torrent.is_single_file);
        assert_eq!(torrent.files.len(), 2);
        assert_eq!(torrent.files[0].path, vec!["dir".to_string(), "a.bin".to_string()]);
        Ok(())
    }

    #[test]
    fn test_from_bytes_missing_info() -> Result<()> {
        let data = encode(&dict(vec![(b"announce".to_vec(), bytes("x"))]))?;
        let res = TorrentInfo::from_bytes(&data);

        assert!(matches!(res, Err(TorrentError::InvalidStructure(_))));
        Ok(())
    }

    #[test]
    fn test_from_bytes_missing_announce() -> Result<()> {
        let data = encode(&dict(vec![(
            b"info".to_vec(),
            dict(vec![
                (b"name".to_vec(), bytes("file.txt")),
                (b"piece length".to_vec(), int(1)),
                (b"pieces".to_vec(), pieces(1)),
                (b"length".to_vec(), int(1)),
            ]),
        )]))?;
        let res = TorrentInfo::from_bytes(&data);

        assert!(matches!(res, Err(TorrentError::BencodeError(_))));
        Ok(())
    }

    #[test]
    fn test_from_bytes_missing_length_and_files() -> Result<()> {
        let data = encode(&dict(vec![
            (b"announce".to_vec(), bytes("x")),
            (
                b"info".to_vec(),
                dict(vec![
                    (b"name".to_vec(), bytes("file")),
                    (b"piece length".to_vec(), int(1)),
                    (b"pieces".to_vec(), pieces(1)),
                ]),
            ),
        ]))?;
        let res = TorrentInfo::from_bytes(&data);

        assert!(matches!(res, Err(TorrentError::InvalidStructure(_))));
        Ok(())
    }

    #[test]
    fn test_from_bytes_invalid_file_entry() -> Result<()> {
        let data = encode(&dict(vec![
            (b"announce".to_vec(), bytes("x")),
            (
                b"info".to_vec(),
                dict(vec![
                    (b"name".to_vec(), bytes("folder")),
                    (b"piece length".to_vec(), int(1)),
                    (b"pieces".to_vec(), pieces(1)),
                    (b"files".to_vec(), list(vec![bytes("bad")])),
                ]),
            ),
        ]))?;
        let res = TorrentInfo::from_bytes(&data);

        assert!(matches!(res, Err(TorrentError::InvalidStructure(_))));
        Ok(())
    }

    #[test]
    fn test_from_bytes_invalid_file_path() -> Result<()> {
        let bad_file = dict(vec![(b"length".to_vec(), int(1))]);
        let data = encode(&dict(vec![
            (b"announce".to_vec(), bytes("x")),
            (
                b"info".to_vec(),
                dict(vec![
                    (b"name".to_vec(), bytes("folder")),
                    (b"piece length".to_vec(), int(1)),
                    (b"pieces".to_vec(), pieces(1)),
                    (b"files".to_vec(), list(vec![bad_file])),
                ]),
            ),
        ]))?;
        let res = TorrentInfo::from_bytes(&data);

        assert!(matches!(res, Err(TorrentError::InvalidStructure(_))));
        Ok(())
    }

    #[test]
    fn test_from_bytes_root_not_dict() -> Result<()> {
        let data = bencode::encode(&int(5))?;
        let res = TorrentInfo::from_bytes(&data);

        assert!(matches!(res, Err(TorrentError::InvalidStructure(_))));
        Ok(())
    }

    #[test]
    fn test_from_file_and_summary() -> Result<()> {
        let data = encode(&sample_single_file())?;
        let path = temp_path("rustatio_test.torrent");
        std::fs::write(&path, &data)?;

        let torrent = TorrentInfo::from_file(&path)?;
        let summary = TorrentInfo::from_file_summary(&path)?;

        assert_eq!(torrent.announce, summary.announce);
        std::fs::remove_file(&path)?;
        Ok(())
    }

    #[test]
    fn test_from_bytes_summary() -> Result<()> {
        let data = encode(&sample_multi_file())?;
        let summary = TorrentInfo::from_bytes_summary(&data)?;

        assert!(summary.files.is_empty());
        assert_eq!(summary.file_count, 2);
        Ok(())
    }

    #[test]
    fn test_get_tracker_url() -> Result<()> {
        let data = encode(&sample_single_file())?;
        let torrent = TorrentInfo::from_bytes(&data)?;

        assert_eq!(torrent.get_tracker_url(), "http://tracker.test/announce");
        Ok(())
    }

    #[test]
    fn test_get_all_tracker_urls() -> Result<()> {
        let data = encode(&sample_multi_file())?;
        let torrent = TorrentInfo::from_bytes(&data)?;
        let urls = torrent.get_all_tracker_urls();

        assert!(urls.contains(&"http://tracker.test/announce".to_string()));
        Ok(())
    }

    #[test]
    fn test_summary_uses_files_len() -> Result<()> {
        let data = encode(&sample_multi_file())?;
        let torrent = TorrentInfo::from_bytes(&data)?;
        let mut with_files = torrent;
        with_files.file_count = 0;
        let summary = with_files.summary();

        assert_eq!(summary.file_count, 2);
        Ok(())
    }

    #[test]
    fn test_without_files() -> Result<()> {
        let data = encode(&sample_multi_file())?;
        let torrent = TorrentInfo::from_bytes(&data)?;
        let trimmed = torrent.without_files();

        assert!(trimmed.files.is_empty());
        assert_eq!(trimmed.file_count, 2);
        Ok(())
    }

    #[test]
    fn test_summary_from_bytes_single_file() -> Result<()> {
        let data = encode(&sample_single_file())?;
        let summary = TorrentSummary::from_bytes(&data)?;

        assert_eq!(summary.total_size, 123);
        assert_eq!(summary.file_count, 1);
        assert!(summary.is_single_file);
        Ok(())
    }

    #[test]
    fn test_summary_from_bytes_multi_file() -> Result<()> {
        let data = encode(&sample_multi_file())?;
        let summary = TorrentSummary::from_bytes(&data)?;

        assert_eq!(summary.total_size, 150);
        assert_eq!(summary.file_count, 2);
        assert!(!summary.is_single_file);
        Ok(())
    }

    #[test]
    fn test_summary_to_info() -> Result<()> {
        let data = encode(&sample_single_file())?;
        let summary = TorrentSummary::from_bytes(&data)?;
        let info = summary.to_info();

        assert!(info.files.is_empty());
        assert_eq!(info.file_count, 1);
        Ok(())
    }

    #[test]
    fn test_info_hash_from_bytes() -> Result<()> {
        let data = encode(&sample_single_file())?;
        let parsed = info_hash(&data)?;

        let info_dict = dict(vec![
            (b"name".to_vec(), bytes("file.txt")),
            (b"piece length".to_vec(), int(16384)),
            (b"pieces".to_vec(), pieces(2)),
            (b"length".to_vec(), int(123)),
        ]);
        let info_bytes = bencode::encode(&info_dict)?;

        let mut hasher = Sha1::new();
        hasher.update(&info_bytes);
        let result = hasher.finalize();

        let mut expected = [0u8; 20];
        expected.copy_from_slice(&result);
        assert_eq!(parsed, expected);
        Ok(())
    }

    #[test]
    fn test_info_hash_missing_info_marker() -> Result<()> {
        let data = bencode::encode(&dict(vec![(b"foo".to_vec(), bytes("bar"))]))?;
        let res = info_hash(&data);

        assert!(matches!(res, Err(TorrentError::InvalidStructure(_))));
        Ok(())
    }
}
