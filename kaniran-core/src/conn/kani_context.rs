//! Rust-only sidecar (CONVENTIONS §1, §2): the port-wide context
//! object holding the live `PgPool` and (eventually) per-DB lazy
//! caches.
//!
//! Replaces three layers of upstream globals in one struct:
//! - `*connection*` (the active connection spec) → [`KaniranContext::pool`].
//! - The `cache`-class registry (`get-cache` / `init-cache` / `ensure`
//!   / `reset-cache`) → typed `OnceCell` fields on `KaniranContext`, added per
//!   cache as the corresponding port lands.
//! - The per-connection variable cache (`*conn-vars*` /
//!   `*conn-var-cache*` / `switch-conn-vars`) → owned per-`KaniranContext` state,
//!   re-acquired by constructing a new `KaniranContext` against another DB.
//!
//! Multi-DB usage (the upstream `with-db` / `let-db` pattern) becomes
//! "construct another `KaniranContext`": each instance owns its own pool and its
//! own caches, no scope-binding macro required.
//!
//! Connection failures are surfaced both as a returned [`Error`] and
//! as a one-line `eprintln!` to stderr, so a failure is visible even
//! when the caller chooses to propagate the error silently. This mirrors
//! the upstream's `dp` / `*debug*` printout pattern; will move to
//! `tracing` when query-level logging lands.

use crate::conn::_star_connection_env_var_star_::DATABASE_URL;
use crate::conn::get_ichiran_connection_env::get_ichiran_connection_env;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("config: {0}")]
    Config(#[from] config::ConfigError),
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("database URL is not set: env var `{0}` is empty or missing")]
    MissingConnection(&'static str),
}

#[derive(Clone)]
pub struct KaniranContext {
    pub pool: PgPool,
}

impl KaniranContext {
    /// Build a context from a Postgres URL.
    ///
    /// On connection failure the underlying [`sqlx::Error`] is logged
    /// to stderr before being returned, so a panicking or
    /// silently-ignoring caller still leaves a trace.
    pub async fn from_url(url: &str) -> Result<Self, Error> {
        let pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_secs(10))
            .connect(url)
            .await
            .map_err(|e| {
                eprintln!("kaniran: failed to connect to database at `{url}`: {e}");
                Error::from(e)
            })?;
        Ok(Self { pool })
    }

    /// Build a context using the [`config`] crate to read a Postgres
    /// URL from the [`DATABASE_URL`] env var (or any layered config
    /// source that supplies the same key).
    ///
    /// Both branches that can fail (missing URL, connection refused)
    /// log a one-line message to stderr before returning the error.
    pub async fn from_env() -> Result<Self, Error> {
        let url = match get_ichiran_connection_env() {
            Ok(Some(u)) => u,
            Ok(None) => {
                eprintln!(
                    "kaniran: database URL is not set — define env var `{DATABASE_URL}`"
                );
                return Err(Error::MissingConnection(DATABASE_URL));
            }
            Err(e) => {
                eprintln!("kaniran: failed to read database URL from config: {e}");
                return Err(e);
            }
        };
        Self::from_url(&url).await
    }
}
