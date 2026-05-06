//! Rust-only sidecar (CONVENTIONS §1, §2): the port-wide context
//! object holding the live `PgPool` and per-DB caches eagerly
//! populated at construction time.
//!
//! Replaces three layers of upstream globals in one struct:
//! - `*connection*` (the active connection spec) → [`KaniranContext::pool`].
//! - The `cache`-class registry (`get-cache` / `init-cache` / `ensure`
//!   / `reset-cache`) → typed fields on `KaniranContext`, populated by
//!   per-cache builder functions called from [`KaniranContext::from_url`].
//! - The per-connection variable cache (`*conn-vars*` /
//!   `*conn-var-cache*` / `switch-conn-vars`) → owned per-`KaniranContext`
//!   state, re-acquired by constructing a new `KaniranContext` against
//!   another DB.
//!
//! Multi-DB usage (the upstream `with-db` / `let-db` pattern) becomes
//! "construct another `KaniranContext`": each instance owns its own pool
//! and its own caches, no scope-binding macro required.
//!
//! ## Sharing
//!
//! Constructors return [`Arc<Self>`]. The struct is intentionally not
//! `Clone` — caches are plain `HashMap`s and we never want a deep
//! clone. Cheap sharing across tasks/threads goes through
//! `Arc::clone`, which bumps a single refcount regardless of how
//! many caches the context owns.
//!
//! Function callsites take `&KaniranContext`. `Arc<KaniranContext>`
//! derefs to `&KaniranContext` automatically, so a caller holding
//! the `Arc` can pass `&ctx` (or `&*ctx`) to any DB-touching fn.
//!
//! Connection failures are surfaced both as a returned [`Error`] and
//! as a one-line `eprintln!` to stderr, so a failure is visible even
//! when the caller chooses to propagate the error silently. This mirrors
//! the upstream's `dp` / `*debug*` printout pattern; will move to
//! `tracing` when query-level logging lands.

use crate::conn::_star_connection_env_var_star_::DATABASE_URL;
use crate::conn::get_ichiran_connection_env::get_ichiran_connection_env;
use crate::dict::_star_counter_cache_star_::{build_counter_cache, CounterCache};
use crate::dict::_star_is_arch_cache_star_::build_is_arch;
use crate::dict::_star_no_conj_data_star_::build_no_conj_data;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::collections::HashSet;
use std::sync::Arc;
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

pub struct KaniranContext {
    pub pool: PgPool,
    /// Upstream `*no-conj-data*` (`dict.lisp:329`). See
    /// [`crate::dict::_star_no_conj_data_star_`].
    pub no_conj_data: HashSet<i32>,
    /// Upstream `*is-arch-cache*` (`dict.lisp:745`). See
    /// [`crate::dict::_star_is_arch_cache_star_`].
    pub is_arch: HashSet<i32>,
    /// Upstream `*counter-cache*` (`dict-counters.lisp:221`). See
    /// [`crate::dict::_star_counter_cache_star_`].
    pub counter_cache: CounterCache,
}

impl KaniranContext {
    /// Build a context from a Postgres URL. Connects the pool and
    /// runs every cache populator before returning, so the result
    /// is fully usable for any DB-backed predicate.
    ///
    /// On connection failure the underlying [`sqlx::Error`] is logged
    /// to stderr before being returned, so a panicking or
    /// silently-ignoring caller still leaves a trace.
    pub async fn from_url(url: &str) -> Result<Arc<Self>, Error> {
        let pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_secs(10))
            .connect(url)
            .await
            .map_err(|e| {
                eprintln!("kaniran: failed to connect to database at `{url}`: {e}");
                Error::from(e)
            })?;
        let no_conj_data = build_no_conj_data(&pool).await?;
        let is_arch = build_is_arch(&pool).await?;
        // counter_cache calls get_counter_readings, which needs a
        // `&KaniranContext`. Build a partial context with empty
        // counter_cache to satisfy the borrow, then swap in the
        // populated map before returning.
        let mut ctx = Self {
            pool,
            no_conj_data,
            is_arch,
            counter_cache: CounterCache::new(),
        };
        ctx.counter_cache = build_counter_cache(&ctx).await?;
        Ok(Arc::new(ctx))
    }

    /// Build a context using the [`config`] crate to read a Postgres
    /// URL from the [`DATABASE_URL`] env var (or any layered config
    /// source that supplies the same key).
    ///
    /// Both branches that can fail (missing URL, connection refused)
    /// log a one-line message to stderr before returning the error.
    pub async fn from_env() -> Result<Arc<Self>, Error> {
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
