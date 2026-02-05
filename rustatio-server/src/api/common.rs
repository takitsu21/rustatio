//! Common API response types.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ApiError {
    success: bool,
    error: String,
}

impl ApiError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: message.into(),
        }
    }

    pub fn response(status: StatusCode, message: impl Into<String>) -> Response {
        (status, Json(Self::new(message))).into_response()
    }
}

#[derive(Serialize, ToSchema)]
pub struct ApiSuccess<T> {
    success: bool,
    data: T,
}

#[derive(Serialize, ToSchema)]
pub struct EmptyData {}

impl<T: Serialize> ApiSuccess<T> {
    pub fn new(data: T) -> Self {
        Self { success: true, data }
    }

    pub fn response(data: T) -> Response
    where
        T: Serialize,
    {
        (StatusCode::OK, Json(Self::new(data))).into_response()
    }
}
