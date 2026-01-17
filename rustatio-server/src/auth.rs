//! Authentication middleware for API token validation.
//!
//! When `AUTH_TOKEN` environment variable is set, all API requests must include
//! a valid `Authorization: Bearer <token>` header or a `?token=<token>` query parameter.
//! The query parameter is needed for SSE connections since EventSource doesn't support headers.

use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::sync::OnceLock;

/// Cached auth token from environment (None = auth disabled)
static AUTH_TOKEN: OnceLock<Option<String>> = OnceLock::new();

/// Get the configured auth token, caching the result
pub fn get_auth_token() -> Option<&'static str> {
    AUTH_TOKEN
        .get_or_init(|| std::env::var("AUTH_TOKEN").ok().filter(|s| !s.is_empty()))
        .as_deref()
}

/// Check if authentication is enabled
pub fn is_auth_enabled() -> bool {
    get_auth_token().is_some()
}

/// Auth error response
#[derive(Serialize)]
struct AuthError {
    success: bool,
    error: String,
    auth_required: bool,
}

impl AuthError {
    fn unauthorized() -> Response {
        (
            StatusCode::UNAUTHORIZED,
            Json(Self {
                success: false,
                error: "Authentication required. Provide Authorization: Bearer <token> header.".into(),
                auth_required: true,
            }),
        )
            .into_response()
    }

    fn forbidden() -> Response {
        (
            StatusCode::FORBIDDEN,
            Json(Self {
                success: false,
                error: "Invalid authentication token.".into(),
                auth_required: true,
            }),
        )
            .into_response()
    }
}

/// Middleware that validates the Authorization header against AUTH_TOKEN.
///
/// If AUTH_TOKEN is not set, all requests are allowed (auth disabled).
/// If AUTH_TOKEN is set, requests must include `Authorization: Bearer <token>` header
/// or a `?token=<token>` query parameter (for SSE connections that don't support headers).
pub async fn auth_middleware(request: Request, next: Next) -> Response {
    // If no auth token configured, allow all requests
    let expected_token = match get_auth_token() {
        Some(token) => token,
        None => return next.run(request).await,
    };

    // First, try Authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    if let Some(header) = auth_header {
        if header.starts_with("Bearer ") {
            let provided_token = &header[7..]; // Skip "Bearer "

            // Constant-time comparison to prevent timing attacks
            if constant_time_eq(provided_token.as_bytes(), expected_token.as_bytes()) {
                return next.run(request).await;
            } else {
                return AuthError::forbidden();
            }
        }
        // Authorization header present but not Bearer scheme - fall through to check query param
    }

    // Try query parameter (for SSE connections)
    if let Some(query) = request.uri().query() {
        for param in query.split('&') {
            if let Some(token_value) = param.strip_prefix("token=") {
                // URL decode the token
                let decoded_token = urlencoding::decode(token_value).unwrap_or_default();
                if constant_time_eq(decoded_token.as_bytes(), expected_token.as_bytes()) {
                    return next.run(request).await;
                } else {
                    return AuthError::forbidden();
                }
            }
        }
    }

    // No valid authentication found
    AuthError::unauthorized()
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hell"));
        assert!(!constant_time_eq(b"", b"a"));
        assert!(constant_time_eq(b"", b""));
    }
}
