//! Network and VPN status endpoints.

use axum::{
    extract::State,
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::{
    common::{ApiError, ApiSuccess, EmptyData},
    ServerState,
};
use crate::services::GluetunAuth;
use rustatio_core::ProxySettings;

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
    pub proxy_enabled: bool,
    pub proxy_url: Option<String>,
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
    let proxy = state.app.proxy_settings().await;
    try_gluetun_detection(
        &GluetunAuth::from_env(),
        state.app.current_forwarded_port(),
        state.app.vpn_port_sync_enabled(),
        listener_status.clone(),
        &proxy,
    )
    .await
    .map_or_else(
        || {
            ApiSuccess::response(no_vpn_status(
                state.app.vpn_port_sync_enabled(),
                listener_status,
                &proxy,
            ))
        },
        ApiSuccess::response,
    )
}

fn no_vpn_status(
    vpn_port_sync_enabled: bool,
    listener_status: rustatio_core::PeerListenerStatus,
    proxy: &ProxySettings,
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
        proxy_enabled: proxy.is_configured(),
        proxy_url: proxy.is_configured().then(|| proxy.url.clone()),
    }
}

async fn try_gluetun_detection(
    auth: &GluetunAuth,
    current_forwarded_port: Option<u16>,
    vpn_port_sync_enabled: bool,
    listener_status: rustatio_core::PeerListenerStatus,
    proxy: &ProxySettings,
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
        proxy_enabled: proxy.is_configured(),
        proxy_url: proxy.is_configured().then(|| proxy.url.clone()),
    })
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/network/status", get(get_network_status))
        .route("/network/proxy", get(get_proxy).put(set_proxy))
        .route("/network/proxy/test", post(test_proxy))
}

#[derive(Serialize, ToSchema)]
pub struct ProxyTestResponse {
    pub ip: String,
}

#[utoipa::path(
    get,
    path = "/network/proxy",
    tag = "network",
    summary = "Get proxy settings",
    description = "Returns the proxy used for tracker announce and scrape requests. Disabled by default.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Proxy settings retrieved", body = ApiSuccess<Object>),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
pub async fn get_proxy(State(state): State<ServerState>) -> Response {
    ApiSuccess::response(state.app.proxy_settings().await)
}

#[utoipa::path(
    put,
    path = "/network/proxy",
    tag = "network",
    summary = "Update proxy settings",
    description = "Validates and applies proxy settings to tracker traffic for new and running instances.",
    security(("bearer_auth" = [])),
    request_body(content = Object, description = "Proxy settings"),
    responses(
        (status = 200, description = "Proxy settings saved", body = ApiSuccess<EmptyData>),
        (status = 400, description = "Invalid proxy settings", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
pub async fn set_proxy(
    State(state): State<ServerState>,
    Json(settings): Json<ProxySettings>,
) -> Response {
    match state.app.set_proxy_settings(settings).await {
        Ok(()) => ApiSuccess::response(EmptyData {}),
        Err(e) => ApiError::response(StatusCode::BAD_REQUEST, e),
    }
}

#[utoipa::path(
    post,
    path = "/network/proxy/test",
    tag = "network",
    summary = "Test proxy connectivity",
    description = "Fetches the public egress IP through the provided proxy settings without saving them.",
    security(("bearer_auth" = [])),
    request_body(content = Object, description = "Proxy settings to test"),
    responses(
        (status = 200, description = "Proxy connection succeeded", body = ApiSuccess<ProxyTestResponse>),
        (status = 400, description = "Proxy connection failed", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError)
    )
)]
pub async fn test_proxy(Json(settings): Json<ProxySettings>) -> Response {
    let proxy = settings.is_configured().then_some(&settings);
    match rustatio_core::proxy::fetch_public_ip(proxy).await {
        Ok(ip) => ApiSuccess::response(ProxyTestResponse { ip }),
        Err(e) => ApiError::response(StatusCode::BAD_REQUEST, e),
    }
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
            &ProxySettings::default(),
        );

        assert!(!status.configured);
        assert!(status.ip.is_empty(), "network status ip should be empty when not configured");
        assert!(!status.is_vpn);
        assert_eq!(status.forwarded_port, None);
        assert_eq!(status.peer_listener_port, Some(51413));
        assert!(status.peer_listener_active);
        assert_eq!(status.peer_listener_error.as_deref(), Some("bind warning"));
        assert!(status.vpn_port_sync_enabled);
        assert!(!status.proxy_enabled);
        assert_eq!(status.proxy_url, None);
    }

    #[test]
    fn no_vpn_status_exposes_configured_proxy() {
        let proxy = ProxySettings {
            enabled: true,
            url: "socks5h://127.0.0.1:1080".to_string(),
            ..ProxySettings::default()
        };

        let status = no_vpn_status(true, rustatio_core::PeerListenerStatus::default(), &proxy);

        assert!(status.proxy_enabled);
        assert_eq!(status.proxy_url.as_deref(), Some("socks5h://127.0.0.1:1080"));
    }
}
