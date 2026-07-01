//! `kaniran-server` — an [axum] HTTP service over the kaniran
//! segmentation/romanization pipeline.
//!
//! The dictionary backend is the memory-mapped rkyv snapshot, loaded once at
//! startup into a shared [`KaniranContext`] and held in [`AppState`]. Each
//! request clones the context's `Arc` (a refcount, not the caches) and runs
//! the pipeline on the blocking pool.
//!
//! # Configuration
//!
//! All via environment variables (see [`Config`]):
//! - `DATABASE_URL` — rkyv snapshot URL, e.g. `memory://…/snapshot.rkyv` (required)
//! - `KANIRAN_HTTP_HOST` — bind interface (default `0.0.0.0`)
//! - `KANIRAN_HTTP_PORT` — bind port (default `3000`)
//! - `KANIRAN_DEFAULT_LIMIT` — default beam width (default `5`)
//! - `KANIRAN_LOG` — `tracing` filter when `RUST_LOG` is unset (default `info`)
//!
//! # Routes
//! - `GET /health` — liveness probe
//! - `GET /segment?text=…&format=…&limit=…`
//! - `POST /segment` — JSON body `{"text": …, "format": …, "limit": …}`
//! - `GET /docs` — Swagger UI, backed by the spec at
//!   `/api-docs/openapi.json`

pub mod config;
mod error;
mod handlers;
mod openapi;

use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use kaniran_core::conn::kani_context::KaniranContext;

use crate::openapi::ApiDoc;

pub use config::Config;
pub use error::ApiError;

/// Shared application state handed to every request.
#[derive(Clone)]
pub struct AppState {
    /// The process-wide dictionary context (rkyv snapshot + caches).
    pub ctx: Arc<KaniranContext>,
    /// Beam width used when a request omits `limit`.
    pub default_limit: usize,
}

/// Assemble the router: routes, Swagger UI, the `tower-http`
/// request-tracing layer, and the shared state.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route(
            "/segment",
            get(handlers::segment_get).post(handlers::segment_post),
        )
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
