use reqwest::header::{HeaderMap, HeaderValue};

const GLUETUN_BASE_URL: &str = "http://localhost:8000";
const GLUETUN_API_KEY_ENV: &str = "GLUETUN_CONTROL_SERVER_API_KEY";
const GLUETUN_API_KEY_HEADER: &str = "X-API-Key";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GluetunAuth {
    api_key: Option<String>,
}

impl GluetunAuth {
    pub fn from_env() -> Self {
        let api_key = std::env::var(GLUETUN_API_KEY_ENV)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        Self { api_key }
    }

    pub fn get(&self, client: &reqwest::Client, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{GLUETUN_BASE_URL}{path}");
        let mut headers = HeaderMap::new();

        if let Some(key) = self.api_key.as_deref() {
            if let Ok(value) = HeaderValue::from_str(key) {
                headers.insert(GLUETUN_API_KEY_HEADER, value);
            }
        }

        client.get(url).headers(headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn from_env_reads_api_key() {
        let guard = env_lock().lock();
        assert!(guard.is_ok(), "failed to acquire env mutex");

        std::env::set_var(GLUETUN_API_KEY_ENV, "secret-key");
        let auth = GluetunAuth::from_env();
        assert_eq!(auth.api_key.as_deref(), Some("secret-key"));

        std::env::remove_var(GLUETUN_API_KEY_ENV);
    }

    #[test]
    fn from_env_ignores_blank_api_key() {
        let guard = env_lock().lock();
        assert!(guard.is_ok(), "failed to acquire env mutex");

        std::env::set_var(GLUETUN_API_KEY_ENV, "   ");
        let auth = GluetunAuth::from_env();
        assert_eq!(auth.api_key, None);

        std::env::remove_var(GLUETUN_API_KEY_ENV);
    }

    #[test]
    fn get_adds_api_key_header_when_present() {
        let client = reqwest::Client::new();
        let auth = GluetunAuth { api_key: Some("secret-key".to_string()) };

        let request = auth.get(&client, "/v1/portforward").build();
        assert!(request.is_ok());

        let request = match request {
            Ok(value) => value,
            Err(e) => panic!("failed to build request: {e}"),
        };

        assert_eq!(request.url().as_str(), "http://localhost:8000/v1/portforward");
        assert_eq!(
            request.headers().get(GLUETUN_API_KEY_HEADER).and_then(|v| v.to_str().ok()),
            Some("secret-key")
        );
    }

    #[test]
    fn get_skips_api_key_header_when_missing() {
        let client = reqwest::Client::new();
        let auth = GluetunAuth::default();

        let request = auth.get(&client, "/v1/publicip/ip").build();
        assert!(request.is_ok());

        let request = match request {
            Ok(value) => value,
            Err(e) => panic!("failed to build request: {e}"),
        };

        assert_eq!(request.url().as_str(), "http://localhost:8000/v1/publicip/ip");
        assert!(request.headers().get(GLUETUN_API_KEY_HEADER).is_none());
    }
}
