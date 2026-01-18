use crate::protocol::bencode;
use crate::torrent::ClientConfig;
use crate::{log_debug, log_error, log_info, log_trace, log_warn};
use reqwest;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TrackerError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Bencode error: {0}")]
    BencodeError(#[from] crate::protocol::bencode::BencodeError),
    #[error("Tracker returned error: {0}")]
    TrackerFailure(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("URL parse error: {0}")]
    UrlError(#[from] url::ParseError),
}

pub type Result<T> = std::result::Result<T, TrackerError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrackerEvent {
    Started,
    Stopped,
    Completed,
    None,
}

impl TrackerEvent {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            TrackerEvent::Started => Some("started"),
            TrackerEvent::Stopped => Some("stopped"),
            TrackerEvent::Completed => Some("completed"),
            TrackerEvent::None => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnnounceRequest {
    pub info_hash: [u8; 20],
    pub peer_id: String,
    pub port: u16,
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub compact: bool,
    pub no_peer_id: bool,
    pub event: TrackerEvent,
    pub ip: Option<String>,
    pub numwant: Option<u32>,
    pub key: Option<String>,
    pub tracker_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnounceResponse {
    /// Interval in seconds between announces
    pub interval: i64,

    /// Minimum announce interval
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_interval: Option<i64>,

    /// Tracker ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracker_id: Option<String>,

    /// Number of seeders
    pub complete: i64,

    /// Number of leechers
    pub incomplete: i64,

    /// Warning message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeResponse {
    pub complete: i64,
    pub incomplete: i64,
    pub downloaded: i64,
    pub name: Option<String>,
}

pub struct TrackerClient {
    client: reqwest::Client,
    client_config: ClientConfig,
}

impl TrackerClient {
    pub fn new(client_config: ClientConfig) -> Result<Self> {
        log_debug!("Creating TrackerClient with User-Agent: {}", client_config.user_agent);

        #[cfg(not(target_arch = "wasm32"))]
        let client = reqwest::Client::builder()
            .user_agent(&client_config.user_agent)
            .timeout(std::time::Duration::from_secs(30))
            .gzip(true)
            .build()?;

        #[cfg(target_arch = "wasm32")]
        let client = reqwest::Client::builder()
            .user_agent(&client_config.user_agent)
            .build()?;

        Ok(TrackerClient { client, client_config })
    }

    /// Send an announce request to the tracker
    pub async fn announce(&self, tracker_url: &str, request: &AnnounceRequest) -> Result<AnnounceResponse> {
        let announce_url = self.build_announce_url(tracker_url, request)?;

        // For WASM, check if proxy is configured
        #[cfg(target_arch = "wasm32")]
        let final_url = {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    if let Ok(Some(proxy)) = storage.get_item("rustatio-proxy-url") {
                        if !proxy.is_empty() {
                            // Encode the announce URL and prepend proxy
                            let encoded = percent_encoding::utf8_percent_encode(
                                &announce_url,
                                percent_encoding::NON_ALPHANUMERIC,
                            )
                            .to_string();
                            format!("{}?url={}", proxy.trim_end_matches('/'), encoded)
                        } else {
                            announce_url.clone()
                        }
                    } else {
                        announce_url.clone()
                    }
                } else {
                    announce_url.clone()
                }
            } else {
                announce_url.clone()
            }
        };

        #[cfg(not(target_arch = "wasm32"))]
        let final_url = announce_url.clone();

        log_info!("Announcing to tracker: {}", tracker_url);
        log_debug!("Full announce URL: {}", final_url);

        let response = self.client.get(&final_url).send().await?;

        let status = response.status();
        log_trace!("Tracker response status: {}", status);

        if !status.is_success() {
            log_error!("Tracker request failed with status: {}", status);
            return Err(TrackerError::HttpError(response.error_for_status().unwrap_err()));
        }

        let body = response.bytes().await?;
        log_debug!("Tracker response: {} bytes", body.len());
        log_trace!("Response body (hex): {:02X?}", &body[..body.len().min(100)]);

        self.parse_announce_response(&body)
    }

    /// Send a scrape request to the tracker
    pub async fn scrape(&self, tracker_url: &str, info_hash: &[u8; 20]) -> Result<ScrapeResponse> {
        let scrape_url = self.build_scrape_url(tracker_url, info_hash)?;

        log_info!("Scraping tracker: {}", scrape_url);

        let response = self.client.get(&scrape_url).send().await?;

        if !response.status().is_success() {
            return Err(TrackerError::HttpError(response.error_for_status().unwrap_err()));
        }

        let body = response.bytes().await?;
        self.parse_scrape_response(&body, info_hash)
    }

    /// Build announce URL with all parameters
    fn build_announce_url(&self, tracker_url: &str, request: &AnnounceRequest) -> Result<String> {
        // Build query parameters manually since info_hash needs special encoding
        let info_hash_encoded: String = request.info_hash.iter().map(|b| format!("%{:02X}", b)).collect();

        let mut params = vec![
            format!("info_hash={}", info_hash_encoded),
            format!("peer_id={}", request.peer_id),
            format!("port={}", request.port),
            format!("uploaded={}", request.uploaded),
            format!("downloaded={}", request.downloaded),
            format!("left={}", request.left),
            format!("compact={}", if request.compact { "1" } else { "0" }),
        ];

        if request.no_peer_id {
            params.push("no_peer_id=1".to_string());
        }

        if let Some(event) = request.event.as_str() {
            params.push(format!("event={}", event));
        }

        if let Some(ref ip) = request.ip {
            params.push(format!("ip={}", ip));
        }

        if let Some(numwant) = request.numwant {
            params.push(format!("numwant={}", numwant));
        }

        if let Some(ref key) = request.key {
            params.push(format!("key={}", key));
        }

        if let Some(ref tracker_id) = request.tracker_id {
            params.push(format!("trackerid={}", tracker_id));
        }

        // Add client-specific parameters
        if self.client_config.supports_crypto {
            params.push("supportcrypto=1".to_string());
        }

        let query_string = params.join("&");
        let separator = if tracker_url.contains('?') { '&' } else { '?' };

        Ok(format!("{}{}{}", tracker_url, separator, query_string))
    }

    /// Build scrape URL from announce URL
    fn build_scrape_url(&self, tracker_url: &str, info_hash: &[u8; 20]) -> Result<String> {
        // Convert announce URL to scrape URL
        let scrape_url = tracker_url.replace("/announce", "/scrape");

        // URL encode info_hash (same format as announce)
        let info_hash_encoded: String = info_hash.iter().map(|b| format!("%{:02X}", b)).collect();

        // Build URL with query parameter
        let separator = if scrape_url.contains('?') { '&' } else { '?' };
        Ok(format!("{}{}info_hash={}", scrape_url, separator, info_hash_encoded))
    }

    /// Parse announce response from bencoded data
    fn parse_announce_response(&self, data: &[u8]) -> Result<AnnounceResponse> {
        log_trace!("Parsing announce response ({} bytes)", data.len());

        let value = match bencode::parse(data) {
            Ok(v) => v,
            Err(_) => {
                // Try to provide a helpful error message about what the tracker returned
                let preview = self.format_response_preview(data);
                log_error!(
                    "Failed to parse tracker response as bencode. Response preview: {}",
                    preview
                );
                return Err(TrackerError::InvalidResponse(format!(
                    "Tracker returned invalid response (not bencode). {}. This usually means: invalid passkey, torrent not registered, IP blocked, or tracker requires login.",
                    preview
                )));
            }
        };
        let dict = match &value {
            serde_bencode::value::Value::Dict(d) => d,
            _ => {
                log_error!("Invalid response: not a dictionary");
                return Err(TrackerError::InvalidResponse("Response is not a dictionary".into()));
            }
        };

        // Check for failure
        if let Some(serde_bencode::value::Value::Bytes(bytes)) = dict.get(b"failure reason".as_ref()) {
            let reason = String::from_utf8_lossy(bytes).to_string();
            log_error!("Tracker returned failure: {}", reason);
            return Err(TrackerError::TrackerFailure(reason));
        }

        // Check for warning
        if let Some(serde_bencode::value::Value::Bytes(bytes)) = dict.get(b"warning message".as_ref()) {
            let warning = String::from_utf8_lossy(bytes).to_string();
            log_warn!("Tracker warning: {}", warning);
        }

        // Extract required fields
        let interval = bencode::get_int(dict, "interval")?;
        let complete = bencode::get_int(dict, "complete").unwrap_or(0);
        let incomplete = bencode::get_int(dict, "incomplete").unwrap_or(0);

        log_debug!(
            "Parsed response: interval={}s, seeders={}, leechers={}",
            interval,
            complete,
            incomplete
        );

        // Extract optional fields
        let min_interval = dict.get(b"min interval".as_ref()).and_then(|v| match v {
            serde_bencode::value::Value::Int(i) => Some(*i),
            _ => None,
        });
        let tracker_id = dict.get(b"tracker id".as_ref()).and_then(|v| match v {
            serde_bencode::value::Value::Bytes(b) => Some(String::from_utf8_lossy(b).to_string()),
            _ => None,
        });
        let warning = dict.get(b"warning message".as_ref()).and_then(|v| match v {
            serde_bencode::value::Value::Bytes(b) => Some(String::from_utf8_lossy(b).to_string()),
            _ => None,
        });

        Ok(AnnounceResponse {
            interval,
            min_interval,
            tracker_id,
            complete,
            incomplete,
            warning,
        })
    }

    /// Parse scrape response from bencoded data
    fn parse_scrape_response(&self, data: &[u8], info_hash: &[u8; 20]) -> Result<ScrapeResponse> {
        let value = match bencode::parse(data) {
            Ok(v) => v,
            Err(_) => {
                let preview = self.format_response_preview(data);
                log_error!(
                    "Failed to parse scrape response as bencode. Response preview: {}",
                    preview
                );
                return Err(TrackerError::InvalidResponse(format!(
                    "Tracker returned invalid scrape response (not bencode). {}",
                    preview
                )));
            }
        };
        let dict = match &value {
            serde_bencode::value::Value::Dict(d) => d,
            _ => return Err(TrackerError::InvalidResponse("Response is not a dictionary".into())),
        };

        // Get the files dictionary
        let files = dict
            .get(b"files".as_ref())
            .and_then(|v| match v {
                serde_bencode::value::Value::Dict(d) => Some(d),
                _ => None,
            })
            .ok_or_else(|| TrackerError::InvalidResponse("Missing 'files' in scrape response".into()))?;

        // Find our torrent's stats (the key is the raw info_hash bytes)
        let stats = files
            .get(info_hash.as_ref())
            .and_then(|v| match v {
                serde_bencode::value::Value::Dict(d) => Some(d),
                _ => None,
            })
            .ok_or_else(|| TrackerError::InvalidResponse("Torrent not found in scrape response".into()))?;

        let complete = bencode::get_int(stats, "complete")?;
        let incomplete = bencode::get_int(stats, "incomplete")?;
        let downloaded = bencode::get_int(stats, "downloaded")?;
        let name = stats.get(b"name".as_ref()).and_then(|v| match v {
            serde_bencode::value::Value::Bytes(b) => Some(String::from_utf8_lossy(b).to_string()),
            _ => None,
        });

        Ok(ScrapeResponse {
            complete,
            incomplete,
            downloaded,
            name,
        })
    }

    /// Format a preview of the response data for error messages
    fn format_response_preview(&self, data: &[u8]) -> String {
        if data.is_empty() {
            return "Response was empty".to_string();
        }

        // Check for common HTML indicators
        let is_html = data.starts_with(b"<!DOCTYPE")
            || data.starts_with(b"<!doctype")
            || data.starts_with(b"<html")
            || data.starts_with(b"<HTML")
            || data.starts_with(b"<?xml");

        // Check for gzip magic bytes
        let is_gzip = data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b;

        if is_html {
            // Try to extract a meaningful snippet from HTML
            let text = String::from_utf8_lossy(&data[..data.len().min(500)]);
            // Try to find title or error message
            if let Some(start) = text.find("<title>").or_else(|| text.find("<TITLE>")) {
                if let Some(end) = text[start..]
                    .find("</title>")
                    .or_else(|| text[start..].find("</TITLE>"))
                {
                    let title = &text[start + 7..start + end];
                    return format!("Received HTML page with title: \"{}\"", title.trim());
                }
            }
            return "Received HTML page instead of tracker response".to_string();
        }

        if is_gzip {
            return "Received gzip-compressed response (tracker may require Accept-Encoding header)".to_string();
        }

        // For other binary data, show a preview
        let preview_len = data.len().min(100);
        let text = String::from_utf8_lossy(&data[..preview_len]);

        // If it looks like text, show it
        if text
            .chars()
            .filter(|c| c.is_ascii_graphic() || c.is_ascii_whitespace())
            .count()
            > preview_len / 2
        {
            format!(
                "Response starts with: \"{}{}\"",
                text.trim(),
                if data.len() > preview_len { "..." } else { "" }
            )
        } else {
            format!("Received {} bytes of binary data", data.len())
        }
    }
}
