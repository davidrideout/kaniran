//! HTTP error type. Each variant maps to a status code and a JSON
//! [`ErrorBody`]; server-side failures are logged.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

/// The JSON body returned for every error response: `{"error": "<message>"}`.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    /// Human-readable description of what went wrong.
    #[schema(example = "`text` must not be empty")]
    pub error: String,
}

/// Anything a request handler can fail with.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// The request was malformed (empty text, unknown format, …) → 400.
    #[error("{0}")]
    BadRequest(String),
    /// The segmentation pipeline returned an error → 500.
    #[error("segmentation failed: {0}")]
    Render(String),
    /// An unexpected internal failure (e.g. a blocking task panicked) → 500.
    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            ApiError::Render(message) | ApiError::Internal(message) => {
                (StatusCode::INTERNAL_SERVER_ERROR, message)
            }
        };
        if status.is_server_error() {
            tracing::error!(error = %message, "request failed");
        }
        (status, Json(ErrorBody { error: message })).into_response()
    }
}
