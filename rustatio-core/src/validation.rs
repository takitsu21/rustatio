use std::fmt::Display;
use std::path::PathBuf;

/// Validation errors
#[derive(Debug)]
pub enum ValidationError {
    InvalidPath(String),
    InvalidFileExtension(String),
    InvalidRange {
        field: String,
        min: f64,
        max: f64,
        value: f64,
    },
    InvalidPort(u16),
    MissingField(String),
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
            ValidationError::InvalidFileExtension(ext) => {
                write!(f, "Invalid file extension. Expected .torrent, got: {}", ext)
            }
            ValidationError::InvalidRange { field, min, max, value } => {
                write!(f, "{} must be between {} and {}, got: {}", field, min, max, value)
            }
            ValidationError::InvalidPort(port) => {
                write!(f, "Invalid port number: {}. Must be between 1024 and 65535", port)
            }
            ValidationError::MissingField(field) => write!(f, "Missing required field: {}", field),
        }
    }
}

/// Validate a torrent file path
pub fn validate_torrent_path(path: &str) -> Result<PathBuf, ValidationError> {
    let path_buf = PathBuf::from(path);

    // Check if file exists
    if !path_buf.exists() {
        return Err(ValidationError::InvalidPath("File does not exist".to_string()));
    }

    // Check if it's a file (not a directory)
    if !path_buf.is_file() {
        return Err(ValidationError::InvalidPath("Path is not a file".to_string()));
    }

    // Check file extension
    if let Some(ext) = path_buf.extension() {
        if ext != "torrent" {
            return Err(ValidationError::InvalidFileExtension(ext.to_string_lossy().to_string()));
        }
    } else {
        return Err(ValidationError::InvalidFileExtension("no extension".to_string()));
    }

    Ok(path_buf)
}

/// Validate upload/download rate (KB/s)
pub fn validate_rate(rate: f64, field_name: &str) -> Result<f64, ValidationError> {
    const MIN_RATE: f64 = 0.0;
    const MAX_RATE: f64 = 1_000_000.0; // 1 TB/s should be more than enough

    if !(MIN_RATE..=MAX_RATE).contains(&rate) {
        return Err(ValidationError::InvalidRange {
            field: field_name.to_string(),
            min: MIN_RATE,
            max: MAX_RATE,
            value: rate,
        });
    }

    Ok(rate)
}

/// Validate port number
pub fn validate_port(port: u16) -> Result<u16, ValidationError> {
    // Ports below 1024 are privileged and shouldn't be used
    if port < 1024 {
        return Err(ValidationError::InvalidPort(port));
    }

    Ok(port)
}

/// Validate update interval (seconds)
pub fn validate_update_interval(interval: u64) -> Result<u64, ValidationError> {
    const MIN_INTERVAL: u64 = 1; // At least 1 second
    const MAX_INTERVAL: u64 = 3600; // At most 1 hour

    if !(MIN_INTERVAL..=MAX_INTERVAL).contains(&interval) {
        return Err(ValidationError::InvalidRange {
            field: "update_interval".to_string(),
            min: MIN_INTERVAL as f64,
            max: MAX_INTERVAL as f64,
            value: interval as f64,
        });
    }

    Ok(interval)
}

/// Validate percentage (0-100)
pub fn validate_percentage(value: f64, field_name: &str) -> Result<f64, ValidationError> {
    if !(0.0..=100.0).contains(&value) {
        return Err(ValidationError::InvalidRange {
            field: field_name.to_string(),
            min: 0.0,
            max: 100.0,
            value,
        });
    }

    Ok(value)
}

// ClientType validation removed - it's an enum so type-safe by design

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_rate() {
        assert!(validate_rate(50.0, "upload_rate").is_ok());
        assert!(validate_rate(0.0, "upload_rate").is_ok());
        assert!(validate_rate(-1.0, "upload_rate").is_err());
        assert!(validate_rate(2_000_000.0, "upload_rate").is_err());
    }

    #[test]
    fn test_validate_port() {
        assert!(validate_port(6881).is_ok());
        assert!(validate_port(65535).is_ok()); // Max valid port
        assert!(validate_port(1023).is_err()); // Below 1024
        assert!(validate_port(1024).is_ok()); // Min valid port
    }

    #[test]
    fn test_validate_percentage() {
        assert!(validate_percentage(50.0, "completion").is_ok());
        assert!(validate_percentage(0.0, "completion").is_ok());
        assert!(validate_percentage(100.0, "completion").is_ok());
        assert!(validate_percentage(-1.0, "completion").is_err());
        assert!(validate_percentage(101.0, "completion").is_err());
    }
}
