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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_validate_rate() {
        // Valid rates
        assert!(validate_rate(50.0, "upload_rate").is_ok());
        assert!(validate_rate(0.0, "upload_rate").is_ok());
        assert!(validate_rate(1000.0, "upload_rate").is_ok());
        assert!(validate_rate(1_000_000.0, "upload_rate").is_ok()); // Max rate

        // Invalid rates
        assert!(validate_rate(-1.0, "upload_rate").is_err());
        assert!(validate_rate(-0.1, "upload_rate").is_err());
        assert!(validate_rate(1_000_001.0, "upload_rate").is_err());
        assert!(validate_rate(2_000_000.0, "upload_rate").is_err());

        // Edge cases
        assert!(validate_rate(f64::EPSILON, "upload_rate").is_ok());
        assert!(validate_rate(999_999.999, "upload_rate").is_ok());
    }

    #[test]
    fn test_validate_port() {
        // Valid ports
        assert!(validate_port(6881).is_ok());
        assert!(validate_port(1024).is_ok()); // Min valid port
        assert!(validate_port(8080).is_ok());
        assert!(validate_port(65535).is_ok()); // Max valid port

        // Invalid ports (privileged)
        assert!(validate_port(0).is_err());
        assert!(validate_port(1).is_err());
        assert!(validate_port(80).is_err());
        assert!(validate_port(443).is_err());
        assert!(validate_port(1023).is_err());
    }

    #[test]
    fn test_validate_percentage() {
        // Valid percentages
        assert!(validate_percentage(0.0, "completion").is_ok());
        assert!(validate_percentage(50.0, "completion").is_ok());
        assert!(validate_percentage(99.9, "completion").is_ok());
        assert!(validate_percentage(100.0, "completion").is_ok());

        // Invalid percentages
        assert!(validate_percentage(-1.0, "completion").is_err());
        assert!(validate_percentage(-0.1, "completion").is_err());
        assert!(validate_percentage(100.1, "completion").is_err());
        assert!(validate_percentage(101.0, "completion").is_err());
        assert!(validate_percentage(1000.0, "completion").is_err());

        // Edge cases
        assert!(validate_percentage(0.0001, "completion").is_ok());
        assert!(validate_percentage(99.9999, "completion").is_ok());
    }

    #[test]
    fn test_validate_update_interval() {
        // Valid intervals
        assert!(validate_update_interval(1).is_ok()); // Min
        assert!(validate_update_interval(30).is_ok());
        assert!(validate_update_interval(60).is_ok());
        assert!(validate_update_interval(300).is_ok());
        assert!(validate_update_interval(3600).is_ok()); // Max

        // Invalid intervals
        assert!(validate_update_interval(0).is_err());
        assert!(validate_update_interval(3601).is_err());
        assert!(validate_update_interval(10000).is_err());
    }

    #[test]
    fn test_validate_torrent_path_nonexistent() {
        let result = validate_torrent_path("/nonexistent/file.torrent");
        assert!(matches!(result, Err(ValidationError::InvalidPath(_))));

        if let Err(ValidationError::InvalidPath(msg)) = result {
            assert!(msg.contains("does not exist"));
        }
    }

    #[test]
    fn test_validate_torrent_path_wrong_extension() {
        // Create a temporary file with wrong extension
        let temp_path = std::env::temp_dir().join("test_file.txt");
        File::create(&temp_path).expect("Failed to create temp file");

        let result = validate_torrent_path(temp_path.to_str().unwrap());
        assert!(matches!(result, Err(ValidationError::InvalidFileExtension(_))));

        if let Err(ValidationError::InvalidFileExtension(ext)) = result {
            assert_eq!(ext, "txt");
        }

        // Clean up
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_validate_torrent_path_no_extension() {
        // Create a temporary file with no extension
        let temp_path = std::env::temp_dir().join("test_file");
        File::create(&temp_path).expect("Failed to create temp file");

        let result = validate_torrent_path(temp_path.to_str().unwrap());
        assert!(matches!(result, Err(ValidationError::InvalidFileExtension(_))));

        if let Err(ValidationError::InvalidFileExtension(ext)) = result {
            assert_eq!(ext, "no extension");
        }

        // Clean up
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_validate_torrent_path_valid() {
        // Create a temporary .torrent file
        let temp_path = std::env::temp_dir().join("test.torrent");
        let mut file = File::create(&temp_path).expect("Failed to create temp file");
        file.write_all(b"dummy content").expect("Failed to write temp file");

        let result = validate_torrent_path(temp_path.to_str().unwrap());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), temp_path);

        // Clean up
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_validate_torrent_path_directory() {
        // Try to validate a directory
        let temp_dir = std::env::temp_dir();

        let result = validate_torrent_path(temp_dir.to_str().unwrap());
        assert!(matches!(result, Err(ValidationError::InvalidPath(_))));

        if let Err(ValidationError::InvalidPath(msg)) = result {
            assert!(msg.contains("not a file"));
        }
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::InvalidPath("test".to_string());
        assert_eq!(format!("{}", err), "Invalid path: test");

        let err = ValidationError::InvalidFileExtension("txt".to_string());
        assert_eq!(
            format!("{}", err),
            "Invalid file extension. Expected .torrent, got: txt"
        );

        let err = ValidationError::InvalidRange {
            field: "rate".to_string(),
            min: 0.0,
            max: 100.0,
            value: 150.0,
        };
        assert_eq!(format!("{}", err), "rate must be between 0 and 100, got: 150");

        let err = ValidationError::InvalidPort(80);
        assert_eq!(
            format!("{}", err),
            "Invalid port number: 80. Must be between 1024 and 65535"
        );

        let err = ValidationError::MissingField("torrent".to_string());
        assert_eq!(format!("{}", err), "Missing required field: torrent");
    }
}
