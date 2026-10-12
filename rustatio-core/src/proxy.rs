//! Global proxy configuration for tracker traffic.

use serde::{Deserialize, Serialize};

/// Proxy applied to tracker announce and scrape requests.
///
/// Settings are optional and additive: missing fields default to a disabled
/// proxy, so older persisted states and API payloads keep working.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProxySettings {
    pub enabled: bool,
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl ProxySettings {
    /// Whether a usable proxy is configured.
    pub fn is_configured(&self) -> bool {
        self.enabled && !self.url.trim().is_empty()
    }

    /// Validate the configured URL. Disabled settings are always valid.
    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        let url = self.url.trim();
        if url.is_empty() {
            return Err("Proxy URL is required when the proxy is enabled".to_string());
        }

        let lower = url.to_ascii_lowercase();
        let supported = ["socks4://", "socks4a://", "socks5://", "socks5h://"];
        if !supported.iter().any(|scheme| lower.starts_with(scheme)) {
            return Err(
                "Proxy URL must start with socks4://, socks4a://, socks5:// or socks5h:// (HTTP proxies are not supported yet)"
                    .to_string(),
            );
        }

        Ok(())
    }
}

/// Flatten an error and its source chain into a single readable message.
///
/// `reqwest::Error`'s top-level display is generic ("error sending request
/// for url ..."); the actionable cause (for example "proxy authorization
/// required" or "credentials not accepted") lives in its source chain.
pub(crate) fn describe(error: &dyn std::error::Error) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut current = Some(error);

    while let Some(err) = current {
        let text = err.to_string();
        if !text.is_empty() && !parts.iter().any(|part| part.contains(&text)) {
            parts.push(text);
        }
        current = err.source();
    }

    parts.join(": ")
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::{describe, ProxySettings};

    /// Total timeout applied to tracker requests.
    const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
    /// Maximum time to establish a connection, including proxy and TLS handshakes.
    const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

    /// Build an HTTP client builder that routes through the configured proxy.
    ///
    /// When no proxy is configured, system proxy environment variables are
    /// still honored by reqwest. Requests always carry a timeout so a stalled
    /// proxy or tracker cannot leave a faker stuck starting or stopping.
    pub fn builder(settings: Option<&ProxySettings>) -> Result<reqwest::ClientBuilder, String> {
        builder_with(settings, REQUEST_TIMEOUT)
    }

    pub(super) fn builder_with(
        settings: Option<&ProxySettings>,
        timeout: std::time::Duration,
    ) -> Result<reqwest::ClientBuilder, String> {
        let mut builder =
            reqwest::Client::builder().timeout(timeout).connect_timeout(CONNECT_TIMEOUT);

        if let Some(settings) = settings.filter(|settings| settings.is_configured()) {
            settings.validate()?;

            let mut proxy = reqwest::Proxy::all(settings.url.trim())
                .map_err(|e| format!("Invalid proxy URL: {e}"))?;

            let username = settings.username.as_deref().map(str::trim).filter(|v| !v.is_empty());
            if let Some(username) = username {
                proxy = proxy.basic_auth(username, settings.password.as_deref().unwrap_or(""));
            }

            builder = builder.proxy(proxy);
        }

        Ok(builder)
    }

    /// Build an HTTP client for tracker traffic.
    pub fn build_client(settings: Option<&ProxySettings>) -> Result<reqwest::Client, String> {
        builder(settings)?.build().map_err(|e| format!("Failed to build HTTP client: {e}"))
    }

    /// Fetch the public egress IP, optionally through the configured proxy.
    pub async fn fetch_public_ip(settings: Option<&ProxySettings>) -> Result<String, String> {
        let client = builder(settings)?
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

        let response = client
            .get("https://api.ipify.org")
            .send()
            .await
            .map_err(|e| format!("Proxy request failed: {}", describe(&e)))?;

        let status = response.status();
        let body = response.text().await.map_err(|e| format!("Failed to read response: {e}"))?;
        let ip = body.trim().to_string();

        if !status.is_success() {
            return Err(format!("Proxy test failed with status {status}"));
        }
        if ip.is_empty() || ip.len() > 64 {
            return Err("Proxy test returned an unexpected response".to_string());
        }

        Ok(ip)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::{build_client, builder, fetch_public_ip};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_proxy_is_not_configured() {
        let settings = ProxySettings::default();
        assert!(!settings.is_configured());
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn enabled_proxy_requires_supported_scheme() {
        let mut settings = ProxySettings {
            enabled: true,
            url: "ftp://proxy:21".to_string(),
            ..Default::default()
        };
        assert!(settings.validate().is_err());

        settings.url = "http://proxy.local:8080".to_string();
        assert!(settings.validate().is_err());

        settings.url = "socks5h://127.0.0.1:1080".to_string();
        assert!(settings.validate().is_ok());
        assert!(settings.is_configured());
    }

    #[test]
    fn enabled_proxy_requires_url() {
        let settings =
            ProxySettings { enabled: true, url: "   ".to_string(), ..Default::default() };
        assert!(!settings.is_configured());
        assert!(settings.validate().is_err());
    }

    #[test]
    fn settings_round_trip_through_json() {
        let settings = ProxySettings {
            enabled: true,
            url: "socks5h://proxy.local:1080".to_string(),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
        };
        let json = serde_json::to_string(&settings).unwrap_or_default();
        let parsed: ProxySettings = serde_json::from_str(&json).unwrap_or_default();
        assert_eq!(parsed, settings);
    }

    #[test]
    fn missing_fields_default_to_disabled() {
        let parsed: ProxySettings = serde_json::from_str("{}").unwrap_or_default();
        assert_eq!(parsed, ProxySettings::default());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn builder_applies_and_rejects_proxy_settings() {
        let settings = ProxySettings {
            enabled: true,
            url: "socks5://127.0.0.1:9".to_string(),
            ..Default::default()
        };
        assert!(builder(Some(&settings)).is_ok());

        let invalid =
            ProxySettings { enabled: true, url: "not-a-url".to_string(), ..Default::default() };
        assert!(builder(Some(&invalid)).is_err());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn client_times_out_on_stalled_server() {
        let Ok(listener) = tokio::net::TcpListener::bind("127.0.0.1:0").await else {
            return;
        };
        let Ok(addr) = listener.local_addr() else {
            return;
        };
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    drop(stream);
                });
            }
        });

        let Ok(client) = super::native::builder_with(None, std::time::Duration::from_millis(150))
            .and_then(|builder| builder.build().map_err(|error| error.to_string()))
        else {
            return;
        };

        let result = client.get(format!("http://{addr}")).send().await;
        assert!(result.is_err());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn describe_joins_error_sources() {
        #[derive(Debug)]
        struct Inner;
        impl std::fmt::Display for Inner {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("credentials not accepted")
            }
        }
        impl std::error::Error for Inner {}

        #[derive(Debug)]
        struct Outer(Inner);
        impl std::fmt::Display for Outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("error connecting to socks proxy")
            }
        }
        impl std::error::Error for Outer {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }

        assert_eq!(
            super::describe(&Outer(Inner)),
            "error connecting to socks proxy: credentials not accepted"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn describe_skips_messages_already_present() {
        #[derive(Debug)]
        struct Inner;
        impl std::fmt::Display for Inner {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("proxy authorization required")
            }
        }
        impl std::error::Error for Inner {}

        #[derive(Debug)]
        struct Outer;
        impl std::fmt::Display for Outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("proxy authorization required")
            }
        }
        impl std::error::Error for Outer {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&Inner)
            }
        }

        assert_eq!(super::describe(&Outer), "proxy authorization required");
    }
}
