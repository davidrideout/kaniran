//! Request error type. Pages are server-rendered, so failures surface as a
//! plain-text 500 — the client script shows the body in the status line
//! instead of swapping it into the page.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use kaniran_core::conn::KaniDbError;

/// Anything a page handler can fail with.
#[derive(Debug, thiserror::Error)]
pub enum DemoError {
    /// The segmentation pipeline returned an error.
    #[error("analysis failed: {0}")]
    Analyze(#[from] KaniDbError),
    /// Template loading or rendering failed.
    #[error("template error: {0}")]
    Template(#[from] minijinja::Error),
    /// An unexpected internal failure (e.g. a blocking task panicked).
    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for DemoError {
    fn into_response(self) -> Response {
        let message = self.to_string();
        tracing::error!(error = %message, "request failed");
        (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
    }
}
