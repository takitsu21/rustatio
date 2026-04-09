//! Network and VPN status endpoints.

use axum::{extract::State, response::Response, routing::get, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::{
    common::{ApiError, ApiSuccess},
    ServerState,
};
use crate::services::GluetunAuth;

#[derive(Serialize, ToSchema)]
pub struct NetworkStatus {
    pub configured: bool,
    pub ip: String,
    pub country: Option<String>,
    pub organization: Option<String>,
    pub is_vpn: bool,
    pub forwarded_port: Option<u16>,
    pub peer_listener_port: Option<u16>,
    pub peer_listener_active: bool,
    pub peer_listener_error: Option<String>,
    pub vpn_port_sync_enabled: bool,
}

#[derive(Deserialize)]
struct GluetunVpnStatus {
    status: String,
}

#[derive(Deserialize)]
struct GluetunPublicIp {
    public_ip: String,
    country: Option<String>,
    organization: Option<String>,
}

#[derive(Deserialize)]
struct GluetunForwardedPort {
    port: u16,
}

#[utoipa::path(
    get,
    path = "/network/status",
    tag = "network",
    summary = "Get network status",
    description = "Returns the current public IP address and VPN connection status. When no VPN is configured, returns a normal response with configured=false.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Network status retrieved", body = ApiSuccess<NetworkStatus>),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
pub async fn get_network_status(State(state): State<ServerState>) -> Response {
    let listener_status = state.app.peer_listener_status().await;
    try_gluetun_detection(
        &GluetunAuth::from_env(),
        state.app.current_forwarded_port(),
        state.app.vpn_port_sync_enabled(),
        listener_status.clone(),
    )
    .await
    .map_or_else(
        || ApiSuccess::response(no_vpn_status(state.app.vpn_port_sync_enabled(), listener_status)),
        ApiSuccess::response,
    )
}

fn no_vpn_status(
    vpn_port_sync_enabled: bool,
    listener_status: rustatio_core::PeerListenerStatus,
) -> NetworkStatus {
    NetworkStatus {
        configured: false,
        ip: String::new(),
        country: None,
        organization: None,
        is_vpn: false,
        forwarded_port: None,
        peer_listener_port: listener_status.bound_port,
        peer_listener_active: listener_status.bound_port.is_some(),
        peer_listener_error: listener_status.last_error,
        vpn_port_sync_enabled,
    }
}

async fn try_gluetun_detection(
    auth: &GluetunAuth,
    current_forwarded_port: Option<u16>,
    vpn_port_sync_enabled: bool,
    listener_status: rustatio_core::PeerListenerStatus,
) -> Option<NetworkStatus> {
    let client =
        reqwest::Client::builder().timeout(std::time::Duration::from_secs(1)).build().ok()?;

    // Get VPN status
    let vpn_status = auth
        .get(&client, "/v1/vpn/status")
        .send()
        .await
        .ok()?
        .json::<GluetunVpnStatus>()
        .await
        .ok()?;

    let is_vpn = vpn_status.status == "running";

    let public_ip = auth
        .get(&client, "/v1/publicip/ip")
        .send()
        .await
        .ok()?
        .json::<GluetunPublicIp>()
        .await
        .ok()?;

    let forwarded_port = match auth.get(&client, "/v1/portforward").send().await {
        Ok(response) => match response.error_for_status() {
            Ok(response) => match response.json::<GluetunForwardedPort>().await {
                Ok(data) if data.port > 0 => Some(data.port),
                _ => current_forwarded_port,
            },
            Err(_) => current_forwarded_port,
        },
        Err(_) => current_forwarded_port,
    };

    Some(NetworkStatus {
        configured: true,
        ip: public_ip.public_ip,
        country: public_ip.country,
        organization: public_ip.organization,
        is_vpn,
        forwarded_port,
        peer_listener_port: listener_status.bound_port,
        peer_listener_active: listener_status.bound_port.is_some(),
        peer_listener_error: listener_status.last_error,
        vpn_port_sync_enabled,
    })
}

pub fn router() -> Router<ServerState> {
    Router::new().route("/network/status", get(get_network_status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_vpn_status_reports_unconfigured_without_error() {
        let status = no_vpn_status(
            true,
            rustatio_core::PeerListenerStatus {
                enabled: true,
                desired_port: Some(51413),
                bound_port: Some(51413),
                active_torrents: 1,
                last_error: Some("bind warning".to_string()),
            },
        );

        assert!(!status.configured);
        assert!(status.ip.is_empty());
        assert!(!status.is_vpn);
        assert_eq!(status.forwarded_port, None);
        assert_eq!(status.peer_listener_port, Some(51413));
        assert!(status.peer_listener_active);
        assert_eq!(status.peer_listener_error.as_deref(), Some("bind warning"));
        assert!(status.vpn_port_sync_enabled);
    }
}
