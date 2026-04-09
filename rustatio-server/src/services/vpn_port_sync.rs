use super::state::AppState;
use crate::services::GluetunAuth;
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::mpsc;

const DEFAULT_INTERVAL_SECS: u64 = 15;

pub struct VpnPortSync {
    shutdown_tx: Option<mpsc::Sender<()>>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VpnPortSyncConfig {
    pub enabled: bool,
    pub interval: Duration,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct GluetunPortForward {
    port: u16,
}

impl VpnPortSyncConfig {
    pub fn from_env() -> Self {
        let enabled = std::env::var("VPN_PORT_SYNC")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false);

        let interval = std::env::var("VPN_PORT_SYNC_INTERVAL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|secs| *secs > 0)
            .unwrap_or(DEFAULT_INTERVAL_SECS);

        Self { enabled, interval: Duration::from_secs(interval) }
    }
}

impl VpnPortSync {
    pub const fn new() -> Self {
        Self { shutdown_tx: None, task_handle: None }
    }

    pub fn start(&mut self, state: AppState, config: VpnPortSyncConfig) {
        if !config.enabled || self.task_handle.is_some() {
            return;
        }

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        let handle = tokio::spawn(sync_loop(state, config, shutdown_rx));
        self.shutdown_tx = Some(shutdown_tx);
        self.task_handle = Some(handle);

        tracing::info!("VPN port sync started (interval={}s)", config.interval.as_secs());
    }

    pub async fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        if let Some(handle) = self.task_handle.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
        }
        tracing::info!("VPN port sync stopped");
    }
}

async fn sync_loop(
    state: AppState,
    config: VpnPortSyncConfig,
    mut shutdown_rx: mpsc::Receiver<()>,
) {
    let auth = GluetunAuth::from_env();
    let client = match reqwest::Client::builder().timeout(Duration::from_secs(2)).build() {
        Ok(client) => client,
        Err(err) => {
            tracing::error!("VPN port sync failed to build HTTP client: {}", err);
            return;
        }
    };

    let mut ticker = tokio::time::interval(config.interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => break,
            _ = ticker.tick() => {
                if let Err(err) = sync_once(&state, &client, &auth).await {
                    tracing::debug!("VPN port sync skipped update: {}", err);
                }
            }
        }
    }
}

async fn sync_once(
    state: &AppState,
    client: &reqwest::Client,
    auth: &GluetunAuth,
) -> Result<(), String> {
    let port = fetch_forwarded_port(client, auth).await?;
    if state.current_forwarded_port() == Some(port) {
        return Ok(());
    }

    let updated = state.apply_vpn_forwarded_port(port).await?;
    tracing::info!(
        "Detected new VPN forwarded port {} and updated {} synced instance(s)",
        port,
        updated
    );

    if updated > 0 {
        state.save_state().await.map_err(|e| format!("Failed to save synced port state: {e}"))?;
    }

    Ok(())
}

async fn fetch_forwarded_port(client: &reqwest::Client, auth: &GluetunAuth) -> Result<u16, String> {
    let response = auth
        .get(client, "/v1/portforward")
        .send()
        .await
        .map_err(|e| format!("Failed to query Gluetun port forward endpoint: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Gluetun port forward endpoint returned HTTP {}", response.status()));
    }

    let payload = response
        .json::<GluetunPortForward>()
        .await
        .map_err(|e| format!("Failed to parse Gluetun port forward response: {e}"))?;

    normalize_port(payload.port)
        .ok_or_else(|| "Gluetun has no forwarded port available yet".to_string())
}

const fn normalize_port(port: u16) -> Option<u16> {
    if port == 0 {
        None
    } else {
        Some(port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_port_ignores_zero() {
        assert_eq!(normalize_port(0), None);
        assert_eq!(normalize_port(51413), Some(51413));
    }

    #[test]
    fn config_defaults_to_disabled() {
        let prev_enabled = std::env::var("VPN_PORT_SYNC").ok();
        let prev_interval = std::env::var("VPN_PORT_SYNC_INTERVAL_SECONDS").ok();

        std::env::remove_var("VPN_PORT_SYNC");
        std::env::remove_var("VPN_PORT_SYNC_INTERVAL_SECONDS");

        let config = VpnPortSyncConfig::from_env();
        assert!(!config.enabled);
        assert_eq!(config.interval, Duration::from_secs(DEFAULT_INTERVAL_SECS));

        match prev_enabled {
            Some(value) => std::env::set_var("VPN_PORT_SYNC", value),
            None => std::env::remove_var("VPN_PORT_SYNC"),
        }
        match prev_interval {
            Some(value) => std::env::set_var("VPN_PORT_SYNC_INTERVAL_SECONDS", value),
            None => std::env::remove_var("VPN_PORT_SYNC_INTERVAL_SECONDS"),
        }
    }
}
