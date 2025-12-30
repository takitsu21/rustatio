use serde::Serialize;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BencodeError {
    #[error("Failed to parse bencode: {0}")]
    ParseError(String),
    #[error("Invalid bencode structure: {0}")]
    InvalidStructure(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, BencodeError>;

/// Parse bencode data from bytes
pub fn parse(data: &[u8]) -> Result<serde_bencode::value::Value> {
    serde_bencode::from_bytes(data).map_err(|e| BencodeError::ParseError(e.to_string()))
}

/// Encode data to bencode format
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_bencode::to_bytes(value).map_err(|e| BencodeError::ParseError(e.to_string()))
}

/// Extract a string value from a bencode dictionary
pub fn get_string(dict: &HashMap<Vec<u8>, serde_bencode::value::Value>, key: &str) -> Result<String> {
    dict.get(key.as_bytes())
        .and_then(|v| match v {
            serde_bencode::value::Value::Bytes(b) => Some(String::from_utf8_lossy(b).to_string()),
            _ => None,
        })
        .ok_or_else(|| BencodeError::InvalidStructure(format!("Missing or invalid key: {}", key)))
}

/// Extract an integer value from a bencode dictionary
pub fn get_int(dict: &HashMap<Vec<u8>, serde_bencode::value::Value>, key: &str) -> Result<i64> {
    dict.get(key.as_bytes())
        .and_then(|v| match v {
            serde_bencode::value::Value::Int(i) => Some(*i),
            _ => None,
        })
        .ok_or_else(|| BencodeError::InvalidStructure(format!("Missing or invalid key: {}", key)))
}

/// Extract bytes value from a bencode dictionary
pub fn get_bytes(dict: &HashMap<Vec<u8>, serde_bencode::value::Value>, key: &str) -> Result<Vec<u8>> {
    dict.get(key.as_bytes())
        .and_then(|v| match v {
            serde_bencode::value::Value::Bytes(b) => Some(b.clone()),
            _ => None,
        })
        .ok_or_else(|| BencodeError::InvalidStructure(format!("Missing or invalid key: {}", key)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_string() {
        let data = b"4:spam";
        let result = parse(data).unwrap();
        match result {
            serde_bencode::value::Value::Bytes(b) => assert_eq!(b, b"spam"),
            _ => panic!("Expected bytes"),
        }
    }

    #[test]
    fn test_parse_integer() {
        let data = b"i42e";
        let result = parse(data).unwrap();
        match result {
            serde_bencode::value::Value::Int(i) => assert_eq!(i, 42),
            _ => panic!("Expected int"),
        }
    }
}
