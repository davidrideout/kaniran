//! `kaniran-demo` — a server-rendered annotated Japanese reader over the
//! kaniran segmentation pipeline.
//!
//! One process, no database: at startup it opens kaniran's memory-mapped
//! rkyv dictionary ([`KaniranContext::from_env`]) and parses the embedded
//! difficulty tables ([`levels`]). Every page is rendered whole on the
//! server with minijinja; the only JavaScript is a thin interactivity layer
//! (fetch-and-swap re-analyze, tooltips, display toggles) that never renders
//! content itself.
//!
//! # Routes
//! - `GET /` — the reader page; `?q=…` analyzes that text (shareable links)
//! - `POST /` — form submit (`q` field) for long pastes; same page back
//! - `/static/*` — stylesheet and the interactivity script
//!
//! [`KaniranContext::from_env`]: kaniran_core::conn::kani_context::KaniranContext::from_env

pub mod config;
mod error;
mod handlers;
mod levels;
mod templates;
mod view;

use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::get;
use axum::Router;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use kaniran_core::conn::kani_context::KaniranContext;

pub use config::Config;
pub use error::DemoError;
pub use levels::Levels;
pub use templates::Templates;

/// Shared, read-only state cloned cheaply into every request.
#[derive(Clone)]
pub struct AppState {
    /// The process-wide dictionary context (rkyv snapshot + caches).
    pub ctx: Arc<KaniranContext>,
    /// Joyo / WaniKani / JLPT difficulty tables.
    pub levels: Arc<Levels>,
    /// The minijinja environment (auto-reloading in front of `templates/`).
    pub templates: Arc<Templates>,
    /// Segmentation beam width.
    pub limit: usize,
}

/// Assemble the router: the reader page, static assets, a generous body
/// limit for large pastes, and request tracing.
pub fn build_router(state: AppState, static_dir: &str) -> Router {
    Router::new()
        .route("/", get(handlers::reader_get).post(handlers::reader_post))
        .nest_service("/static", ServeDir::new(static_dir))
        // Allow large pastes (default body limit is only 2 MB).
        .layer(DefaultBodyLimit::max(16 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
