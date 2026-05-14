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
use crate::dict::_star_substring_hash_star_::SubstringHash;
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

#[derive(Clone)]
pub struct KaniranContext {
    pub pool: PgPool,
    /// Upstream `*no-conj-data*` (`dict.lisp:329`). See
    /// [`crate::dict::_star_no_conj_data_star_`].
    pub no_conj_data: Arc<HashSet<i32>>,
    /// Upstream `*is-arch-cache*` (`dict.lisp:745`). See
    /// [`crate::dict::_star_is_arch_cache_star_`].
    pub is_arch: Arc<HashSet<i32>>,
    /// Upstream `*counter-cache*` (`dict-counters.lisp:221`). See
    /// [`crate::dict::_star_counter_cache_star_`].
    pub counter_cache: Arc<CounterCache>,
    /// Upstream `*suffix-cache*` (`dict-grammar.lisp:5`). See
    /// [`crate::dict::_star_suffix_cache_star_`].
    pub suffix_cache: Arc<SuffixCache>,
    /// Upstream `*suffix-class*` (`dict-grammar.lisp:6`). See
    /// [`crate::dict::_star_suffix_class_star_`].
    pub suffix_class: Arc<SuffixClass>,
    /// Upstream `*reading-cache*` (`kanji.lisp:199`). See
    /// [`crate::kanji::_star_reading_cache_star_`].
    pub reading_cache: Arc<ReadingCache>,

    /// Upstream `*disable-hints*` (`dict.lisp:78`) — recursion guard
    /// for the `simple-text :around` method on
    /// [`crate::dict::get_kana::get_kana`]. Rebound to `true` at two
    /// sites: `dict.lisp:82` (the `:around` body, around the call to
    /// `get-hint`) and `dict-split.lisp:909` (`check-easy-hints`,
    /// around the per-row `true-kana` call). Default `false` matches
    /// the upstream `(defvar … nil)` initform.
    ///
    /// Lives on ctx (not as a `tokio::task_local!` or `thread_local!`)
    /// so the binding survives crossing rayon / `tokio::spawn`
    /// boundaries: spawned closures capture `&ctx2` by reference and
    /// the borrow-checker enforces propagation, with no scope
    /// snapshot/restore obligation at each parallel boundary.
    pub disable_hints: bool,

    /// Upstream `*substring-hash*` (`dict.lisp:487`) — per-call-tree
    /// lookup cache short-circuiting [`crate::dict::find_word::find_word`].
    /// Rebound only inside `find-word-full`'s nested-find loop
    /// (`dict.lisp:1090-1092`); `None` outside that scope matches the
    /// upstream `(defparameter … nil)` initform. Wrapped in `Arc` so
    /// the rebind clone is cheap.
    pub substring_hash: Option<Arc<SubstringHash>>,
}

impl KaniranContext {
    /// `(let ((*disable-hints* v)) …)` — return a sibling context with
    /// the hint-recursion guard rebound. Cheap because the cache
    /// fields are `Arc`-shared.
    pub fn with_disable_hints(&self, v: bool) -> Self {
        Self { disable_hints: v, ..self.clone() }
    }

    /// `(let ((*substring-hash* h)) …)` — return a sibling context
    /// with the find-word short-circuit cache populated.
    pub fn with_substring_hash(&self, h: Arc<SubstringHash>) -> Self {
        Self { substring_hash: Some(h), ..self.clone() }
    }
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
        let no_conj_data = Arc::new(build_no_conj_data(&pool).await?);
        let is_arch = Arc::new(build_is_arch(&pool).await?);
        // counter_cache + suffix_cache populators call DB-touching fns
        // that take &KaniranContext — build a partial ctx first, then
        // swap the populated maps in.
        let mut ctx = Self {
            pool,
            no_conj_data,
            is_arch,
            counter_cache: Arc::new(CounterCache::new()),
            suffix_cache: Arc::new(SuffixCache::new()),
            suffix_class: Arc::new(SuffixClass::new()),
            reading_cache: Arc::new(new_reading_cache()),
            disable_hints: false,
            substring_hash: None,
        };
        ctx.counter_cache = Arc::new(build_counter_cache(&ctx).await?);
        let (suffix_cache, suffix_class) = build_suffix_caches(&ctx).await?;
        ctx.suffix_cache = Arc::new(suffix_cache);
        ctx.suffix_class = Arc::new(suffix_class);
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
