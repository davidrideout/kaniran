//! Rust-only sidecar (CONVENTIONS §1, §2): the port-wide context.
//!
//! Replaces three layers of upstream globals:
//! - `*connection*` → [`KaniranContext::pool`].
//! - The `cache`-class registry (`get-cache` / `init-cache` / `ensure`
//!   / `reset-cache`) → typed fields populated by per-cache builders
//!   from [`KaniranContext::from_url`].
//! - The per-connection variable cache (`*conn-vars*` /
//!   `*conn-var-cache*` / `switch-conn-vars`) → owned per-context
//!   state. Multi-DB use (`with-db` / `let-db`) = construct another
//!   `KaniranContext`.
//!
//! `Error` failures are also `eprintln!`ed before propagating; mirrors
//! upstream's `dp` / `*debug*`. Will move to `tracing` when query-level
//! logging lands.

use crate::conn::_star_connection_env_var_star_::DATABASE_URL;
use crate::conn::get_ichiran_connection_env::get_ichiran_connection_env;
use crate::dict::_star_counter_cache_star_::{build_counter_cache, CounterCache};
use crate::dict::_star_is_arch_cache_star_::build_is_arch;
use crate::dict::_star_no_conj_data_star_::build_no_conj_data;
use crate::dict::_star_suffix_cache_star_::SuffixCache;
use crate::dict::_star_suffix_class_star_::SuffixClass;
use crate::dict::init_suffixes_thread::build_suffix_caches;
use crate::kanji::_star_reading_cache_star_::{new_reading_cache, ReadingCache};
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
    /// Upstream `*suffix-cache*` (`dict-grammar.lisp:5`). See
    /// [`crate::dict::_star_suffix_cache_star_`].
    pub suffix_cache: SuffixCache,
    /// Upstream `*suffix-class*` (`dict-grammar.lisp:6`). See
    /// [`crate::dict::_star_suffix_class_star_`].
    pub suffix_class: SuffixClass,
    /// Upstream `*reading-cache*` (`kanji.lisp:199`). See
    /// [`crate::kanji::_star_reading_cache_star_`].
    pub reading_cache: ReadingCache,
}

impl KaniranContext {
    /// Connect the pool and run every cache populator before returning.
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
        // counter_cache + suffix_cache populators call DB-touching fns
        // that take &KaniranContext — build a partial ctx first, then
        // swap the populated maps in.
        let mut ctx = Self {
            pool,
            no_conj_data,
            is_arch,
            counter_cache: CounterCache::new(),
            suffix_cache: SuffixCache::new(),
            suffix_class: SuffixClass::new(),
            reading_cache: new_reading_cache(),
        };
        ctx.counter_cache = build_counter_cache(&ctx).await?;
        let (suffix_cache, suffix_class) = build_suffix_caches(&ctx).await?;
        ctx.suffix_cache = suffix_cache;
        ctx.suffix_class = suffix_class;
        Ok(Arc::new(ctx))
    }

    /// Read a Postgres URL via [`config::Config`] (file + env layered)
    /// and build the context.
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
