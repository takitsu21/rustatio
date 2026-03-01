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
#[allow(clippy::implicit_hasher)]
pub fn get_string(
    dict: &HashMap<Vec<u8>, serde_bencode::value::Value>,
    key: &str,
) -> Result<String> {
    dict.get(key.as_bytes())
        .and_then(|v| match v {
            serde_bencode::value::Value::Bytes(b) => Some(String::from_utf8_lossy(b).to_string()),
            _ => None,
        })
        .ok_or_else(|| BencodeError::InvalidStructure(format!("Missing or invalid key: {key}")))
}

/// Extract an integer value from a bencode dictionary
#[allow(clippy::implicit_hasher)]
pub fn get_int(dict: &HashMap<Vec<u8>, serde_bencode::value::Value>, key: &str) -> Result<i64> {
    dict.get(key.as_bytes())
        .and_then(|v| match v {
            serde_bencode::value::Value::Int(i) => Some(*i),
            _ => None,
        })
        .ok_or_else(|| BencodeError::InvalidStructure(format!("Missing or invalid key: {key}")))
}

/// Extract bytes value from a bencode dictionary
#[allow(clippy::implicit_hasher)]
pub fn get_bytes(
    dict: &HashMap<Vec<u8>, serde_bencode::value::Value>,
    key: &str,
) -> Result<Vec<u8>> {
    dict.get(key.as_bytes())
        .and_then(|v| match v {
            serde_bencode::value::Value::Bytes(b) => Some(b.clone()),
            _ => None,
        })
        .ok_or_else(|| BencodeError::InvalidStructure(format!("Missing or invalid key: {key}")))
}

/// Extract byte length from a bencode dictionary
#[allow(clippy::implicit_hasher)]
pub fn get_bytes_len(
    dict: &HashMap<Vec<u8>, serde_bencode::value::Value>,
    key: &str,
) -> Result<usize> {
    dict.get(key.as_bytes())
        .and_then(|v| match v {
            serde_bencode::value::Value::Bytes(b) => Some(b.len()),
            _ => None,
        })
        .ok_or_else(|| BencodeError::InvalidStructure(format!("Missing or invalid key: {key}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_bencode::value::Value;

    fn map(entries: Vec<(Vec<u8>, Value)>) -> HashMap<Vec<u8>, Value> {
        let mut dict = HashMap::new();
        for (key, value) in entries {
            dict.insert(key, value);
        }
        dict
    }

    #[test]
    fn test_parse_simple_string() -> Result<()> {
        let data = b"4:spam";
        let result = parse(data)?;
        assert!(matches!(result, Value::Bytes(ref b) if b == b"spam"));
        Ok(())
    }

    #[test]
    fn test_parse_integer() -> Result<()> {
        let data = b"i42e";
        let result = parse(data)?;
        assert!(matches!(result, Value::Int(42)));
        Ok(())
    }

    #[test]
    fn test_parse_invalid() {
        let result = parse(b"notbencode");
        assert!(matches!(result, Err(BencodeError::ParseError(_))));
    }

    #[test]
    fn test_encode_roundtrip() -> Result<()> {
        let value = Value::Dict(map(vec![
            (b"foo".to_vec(), Value::Bytes(b"bar".to_vec())),
            (b"num".to_vec(), Value::Int(7)),
        ]));
        let data = encode(&value)?;
        let decoded = parse(&data)?;

        assert_eq!(decoded, value);
        Ok(())
    }

    #[test]
    fn test_get_string_ok() -> Result<()> {
        let dict = map(vec![(b"name".to_vec(), Value::Bytes(b"test".to_vec()))]);
        let value = get_string(&dict, "name")?;
        assert_eq!(value, "test");
        Ok(())
    }

    #[test]
    fn test_get_string_missing() {
        let dict = HashMap::new();
        let result = get_string(&dict, "name");
        assert!(matches!(result, Err(BencodeError::InvalidStructure(_))));
    }

    #[test]
    fn test_get_string_wrong_type() {
        let dict = map(vec![(b"name".to_vec(), Value::Int(1))]);
        let result = get_string(&dict, "name");
        assert!(matches!(result, Err(BencodeError::InvalidStructure(_))));
    }

    #[test]
    fn test_get_int_ok() -> Result<()> {
        let dict = map(vec![(b"num".to_vec(), Value::Int(9))]);
        let value = get_int(&dict, "num")?;
        assert_eq!(value, 9);
        Ok(())
    }

    #[test]
    fn test_get_int_missing() {
        let dict = HashMap::new();
        let result = get_int(&dict, "num");
        assert!(matches!(result, Err(BencodeError::InvalidStructure(_))));
    }

    #[test]
    fn test_get_int_wrong_type() {
        let dict = map(vec![(b"num".to_vec(), Value::Bytes(b"x".to_vec()))]);
        let result = get_int(&dict, "num");
        assert!(matches!(result, Err(BencodeError::InvalidStructure(_))));
    }

    #[test]
    fn test_get_bytes_ok() -> Result<()> {
        let dict = map(vec![(b"data".to_vec(), Value::Bytes(b"abc".to_vec()))]);
        let value = get_bytes(&dict, "data")?;
        assert_eq!(value, b"abc".to_vec());
        Ok(())
    }

    #[test]
    fn test_get_bytes_missing() {
        let dict = HashMap::new();
        let result = get_bytes(&dict, "data");
        assert!(matches!(result, Err(BencodeError::InvalidStructure(_))));
    }

    #[test]
    fn test_get_bytes_wrong_type() {
        let dict = map(vec![(b"data".to_vec(), Value::Int(1))]);
        let result = get_bytes(&dict, "data");
        assert!(matches!(result, Err(BencodeError::InvalidStructure(_))));
    }

    #[test]
    fn test_get_bytes_len_ok() -> Result<()> {
        let dict = map(vec![(b"data".to_vec(), Value::Bytes(b"abcd".to_vec()))]);
        let value = get_bytes_len(&dict, "data")?;
        assert_eq!(value, 4);
        Ok(())
    }

    #[test]
    fn test_get_bytes_len_missing() {
        let dict = HashMap::new();
        let result = get_bytes_len(&dict, "data");
        assert!(matches!(result, Err(BencodeError::InvalidStructure(_))));
    }

    #[test]
    fn test_get_bytes_len_wrong_type() {
        let dict = map(vec![(b"data".to_vec(), Value::Int(1))]);
        let result = get_bytes_len(&dict, "data");
        assert!(matches!(result, Err(BencodeError::InvalidStructure(_))));
    }
}
