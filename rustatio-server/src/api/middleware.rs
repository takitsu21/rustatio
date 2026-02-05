//! Authentication middleware for API token validation.

use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::sync::OnceLock;

static AUTH_TOKEN: OnceLock<Option<String>> = OnceLock::new();

pub fn get_auth_token() -> Option<&'static str> {
    AUTH_TOKEN
        .get_or_init(|| std::env::var("AUTH_TOKEN").ok().filter(|s| !s.is_empty()))
        .as_deref()
}

pub fn is_auth_enabled() -> bool {
    get_auth_token().is_some()
}

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

/// Validates Authorization header or query token against AUTH_TOKEN.
/// If AUTH_TOKEN is not set, all requests are allowed.
pub async fn auth_middleware(request: Request, next: Next) -> Response {
    let expected_token = match get_auth_token() {
        Some(token) => token,
        None => return next.run(request).await,
    };

    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    if let Some(header) = auth_header {
        if let Some(provided_token) = header.strip_prefix("Bearer ") {
            if constant_time_eq(provided_token.as_bytes(), expected_token.as_bytes()) {
                return next.run(request).await;
            } else {
                return AuthError::forbidden();
            }
        }
    }

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

    AuthError::unauthorized()
}

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
