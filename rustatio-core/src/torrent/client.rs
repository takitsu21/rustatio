use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClientType {
    #[serde(rename = "utorrent")]
    UTorrent,
    #[serde(rename = "qbittorrent")]
    QBittorrent,
    #[serde(rename = "transmission")]
    Transmission,
    #[serde(rename = "deluge")]
    Deluge,
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub client_type: ClientType,
    pub version: String,
    pub peer_id_prefix: String,
    pub user_agent: String,
    pub http_version: HttpVersion,
    pub num_want: u32,
    pub supports_compact: bool,
    pub supports_crypto: bool,
}

#[derive(Debug, Clone)]
pub enum HttpVersion {
    Http10,
    Http11,
}

impl ClientConfig {
    /// Get configuration for a specific client
    pub fn get(client_type: ClientType, version: Option<String>) -> Self {
        match client_type {
            ClientType::UTorrent => Self::utorrent(version),
            ClientType::QBittorrent => Self::qbittorrent(version),
            ClientType::Transmission => Self::transmission(version),
            ClientType::Deluge => Self::deluge(version),
        }
    }

    /// uTorrent client configuration
    fn utorrent(version: Option<String>) -> Self {
        let version = version.unwrap_or_else(|| "3.5.5".to_string());
        let version_code = version.replace('.', "");

        ClientConfig {
            client_type: ClientType::UTorrent,
            version: version.clone(),
            peer_id_prefix: format!("-UT{}-", &version_code[..4]),
            user_agent: format!("uTorrent/{}", version_code),
            http_version: HttpVersion::Http11,
            num_want: 200,
            supports_compact: true,
            supports_crypto: true,
        }
    }

    /// qBittorrent client configuration
    fn qbittorrent(version: Option<String>) -> Self {
        let version = version.unwrap_or_else(|| "5.1.4".to_string());
        let parts: Vec<&str> = version.split('.').collect();
        let version_code = if parts.len() >= 3 {
            format!("{}{}{}", parts[0], parts[1], parts[2])
        } else {
            "514".to_string()
        };

        // Pad to exactly 4 characters
        let padded_version = version_code.pad_to_width_with_char(4, '0');

        ClientConfig {
            client_type: ClientType::QBittorrent,
            version: version.clone(),
            peer_id_prefix: format!("-qB{}-", padded_version),
            user_agent: format!("qBittorrent/{}", version),
            http_version: HttpVersion::Http11,
            num_want: 200,
            supports_compact: true,
            supports_crypto: true,
        }
    }

    /// Transmission client configuration
    fn transmission(version: Option<String>) -> Self {
        let version = version.unwrap_or_else(|| "4.0.5".to_string());
        let parts: Vec<&str> = version.split('.').collect();
        let version_code = if parts.len() >= 2 {
            format!("{}{}", parts[0], parts[1].pad_to_width_with_char(2, '0'))
        } else {
            "400".to_string()
        };

        // Pad to exactly 4 characters
        let padded_version = version_code.pad_to_width_with_char(4, '0');

        ClientConfig {
            client_type: ClientType::Transmission,
            version: version.clone(),
            peer_id_prefix: format!("-TR{}-", padded_version),
            user_agent: format!("Transmission/{}", version),
            http_version: HttpVersion::Http11,
            num_want: 80,
            supports_compact: true,
            supports_crypto: true,
        }
    }

    /// Deluge client configuration
    fn deluge(version: Option<String>) -> Self {
        let version = version.unwrap_or_else(|| "2.1.1".to_string());
        let parts: Vec<&str> = version.split('.').collect();
        let version_code = if parts.len() >= 3 {
            format!("{}{}{}", parts[0], parts[1], parts[2])
        } else {
            "211".to_string()
        };

        // Pad to exactly 4 characters
        let padded_version = version_code.pad_to_width_with_char(4, '0');

        ClientConfig {
            client_type: ClientType::Deluge,
            version: version.clone(),
            peer_id_prefix: format!("-DE{}-", padded_version),
            user_agent: format!("Deluge/{}", version),
            http_version: HttpVersion::Http11,
            num_want: 200,
            supports_compact: true,
            supports_crypto: true,
        }
    }

    /// Generate a random peer ID based on this client config
    pub fn generate_peer_id(&self) -> String {
        let mut rng = rand::rng();
        let random_suffix: String = (0..12)
            .map(|_| {
                let chars = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
                chars[rng.random_range(0..chars.len())] as char
            })
            .collect();

        format!("{}{}", self.peer_id_prefix, random_suffix)
    }

    /// Generate a random key (8 hex characters)
    pub fn generate_key() -> String {
        let mut rng = rand::rng();
        (0..8).map(|_| format!("{:X}", rng.random_range(0..16))).collect()
    }
}

trait PadString {
    fn pad_to_width_with_char(&self, width: usize, ch: char) -> String;
}

impl PadString for str {
    fn pad_to_width_with_char(&self, width: usize, ch: char) -> String {
        if self.len() >= width {
            self[..width].to_string()
        } else {
            format!("{}{}", self, ch.to_string().repeat(width - self.len()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_id_generation() {
        let config = ClientConfig::get(ClientType::QBittorrent, None);
        let peer_id = config.generate_peer_id();
        assert_eq!(peer_id.len(), 20);
        assert!(peer_id.starts_with("-qB"));
    }

    #[test]
    fn test_key_generation() {
        let key = ClientConfig::generate_key();
        assert_eq!(key.len(), 8);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
