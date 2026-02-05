//! Network and VPN status endpoints.

use axum::{http::StatusCode, response::Response, routing::get, Router};
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
pub async fn get_network_status() -> Response {
    match try_gluetun_detection().await {
        Some(status) => ApiSuccess::response(status),
        None => ApiError::response(
            StatusCode::SERVICE_UNAVAILABLE,
            "Gluetun not available. Network status requires Docker with gluetun VPN container.",
        ),
    }
}

async fn try_gluetun_detection() -> Option<NetworkStatus> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1000))
        .build()
        .ok()?;

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

    Some(NetworkStatus {
        ip: public_ip.public_ip,
        country: public_ip.country,
        organization: public_ip.organization,
        is_vpn,
    })
}

pub fn router() -> Router<ServerState> {
    Router::new().route("/network/status", get(get_network_status))
}
