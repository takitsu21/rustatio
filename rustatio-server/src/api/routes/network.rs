//! Network and VPN status endpoints.

use axum::{extract::State, http::StatusCode, response::Response, routing::get, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::{
    common::{ApiError, ApiSuccess},
    ServerState,
};

#[derive(Serialize, ToSchema)]
pub struct NetworkStatus {
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
    description = "Returns the current public IP address and VPN connection status. Requires gluetun container to be running.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Network status retrieved", body = ApiSuccess<NetworkStatus>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 503, description = "Gluetun not available", body = ApiError)
    )
)]
pub async fn get_network_status(State(state): State<ServerState>) -> Response {
    let listener_status = state.app.peer_listener_status().await;
    try_gluetun_detection(
        state.app.current_forwarded_port(),
        state.app.vpn_port_sync_enabled(),
        listener_status,
    )
    .await
    .map_or_else(
        || {
            ApiError::response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Gluetun not available. Network status requires Docker with gluetun VPN container.",
            )
        },
        ApiSuccess::response,
    )
}

async fn try_gluetun_detection(
    current_forwarded_port: Option<u16>,
    vpn_port_sync_enabled: bool,
    listener_status: rustatio_core::PeerListenerStatus,
) -> Option<NetworkStatus> {
    let client =
        reqwest::Client::builder().timeout(std::time::Duration::from_millis(1000)).build().ok()?;

    // Get VPN status
    let vpn_status = client
        .get("http://localhost:8000/v1/vpn/status")
        .send()
        .await
        .ok()?
        .json::<GluetunVpnStatus>()
        .await
        .ok()?;

    let is_vpn = vpn_status.status == "running";

    let public_ip = client
        .get("http://localhost:8000/v1/publicip/ip")
        .send()
        .await
        .ok()?
        .json::<GluetunPublicIp>()
        .await
        .ok()?;

    let forwarded_port = match client.get("http://localhost:8000/v1/portforward").send().await {
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
