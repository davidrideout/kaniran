//! Rust-only sidecar: the port-wide context, holding the dictionary
//! lookup backend and every populated cache. Replaces upstream's
//! `*connection*`, the `cache`-class registry, and the per-connection
//! variable cache; multi-DB use means constructing another
//! `KaniranContext`.
//!
//! Since the async-removal proof-of-concept the only backend is the
//! memory-mapped rkyv snapshot (`DATABASE_URL=memory://<archive>`); the
//! Postgres connection pool is gone and construction is synchronous.

use crate::conn::_star_connection_env_var_star_::DATABASE_URL;
use crate::conn::get_ichiran_connection_env::get_ichiran_connection_env;
use crate::conn::kani_backend::KaniStore;
use crate::dict::counters::dispatchers::{build_counter_cache, CounterCache};
use crate::dict::scoring::score::build_is_arch;
use crate::dict::conj::build_no_conj_data;
use crate::dict::split::split_map::SplitMapKind;
use crate::dict::path::SubstringHash;
use crate::dict::grammar::suffix::constants::SuffixCache;
use crate::dict::grammar::suffix::constants::SuffixClass;
use crate::dict::word_info::SuffixMapTemp;
use crate::dict::grammar::suffix::init::build_suffix_caches;
use crate::kanji::helpers::{new_reading_cache, ReadingCache};
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("config: {0}")]
    Config(#[from] config::ConfigError),
    #[error("database: {0}")]
    Database(#[from] crate::conn::KaniDbError),
    #[error("database URL is not set: env var `{0}` is empty or missing")]
    MissingConnection(&'static str),
    #[error("rkyv snapshot: {0}")]
    Snapshot(String),
}

/// The process-lifetime, immutable half of [`KaniranContext`]: the
/// lookup backend and every populated cache. Held behind a single `Arc`
/// on the context so a per-call binding rebind
/// ([`KaniranContext::with_disable_hints`] and siblings) bumps one
/// refcount instead of one per cache — cloning every cache Arc on each
/// rebind was the segmenter's dominant multi-thread serialization point
/// (contended refcount atomics on the shared caches).
pub struct KaniranShared {
    #[cfg(feature = "postgres")]
    /// Postgres connection pool when the store is the Postgres backend
    /// (`None` for the rkyv snapshot). Used by the build-time loaders.
    pub pool: Option<sqlx::PgPool>,
    /// Dictionary lookup backend (the memory-mapped rkyv snapshot). All
    /// runtime lookup-serving queries go through here.
    pub store: KaniStore,
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
}

#[derive(Clone)]
pub struct KaniranContext {
    /// Process-lifetime shared state — store and every cache. One
    /// `Arc`, so a per-call binding rebind clones a single refcount.
    /// Its fields are reachable directly as `ctx.<name>` through the
    /// [`Deref`](std::ops::Deref) to [`KaniranShared`].
    pub shared: Arc<KaniranShared>,

    /// Upstream `*disable-hints*` (`dict.lisp:78`) — recursion guard
    /// for the `simple-text :around` method on
    /// [`crate::dict::accessors::get_kana`]. Rebound to `true` at two
    /// sites: `dict.lisp:82` (the `:around` body, around the call to
    /// `get-hint`) and `dict-split.lisp:909` (`check-easy-hints`,
    /// around the per-row `true-kana` call). Default `false` matches
    /// the upstream `(defvar … nil)` initform.
    pub disable_hints: bool,

    /// Upstream `*substring-hash*` (`dict.lisp:487`) — per-call-tree
    /// lookup cache short-circuiting [`crate::dict::readings::find_word`].
    /// Rebound only inside `find-word-full`'s nested-find loop
    /// (`dict.lisp:1090-1092`); `None` outside that scope matches the
    /// upstream `(defparameter … nil)` initform. Wrapped in `Arc` so
    /// the rebind clone is cheap.
    pub substring_hash: Option<Arc<SubstringHash>>,

    /// Upstream `*suffix-map-temp*` (`dict.lisp:1049`) — caller-scoped
    /// suffix-candidate cache keyed by end-position. See
    /// [`crate::dict::_star_suffix_map_temp_star_`]. Rebound by
    /// `join-substring-words*` (`dict.lisp:1090`), `find-word-info`
    /// (`dict.lisp:1851`), and the `def-simple-suffix` /
    /// `def-abbr-suffix` / `suffix-sou-base` / `suffix-garu` bodies in
    /// `dict-grammar.lisp`. `None` outside those scopes matches the
    /// upstream `(defvar … nil)` initform.
    pub suffix_map_temp: Option<Arc<SuffixMapTemp>>,

    /// Upstream `*suffix-next-end*` (`dict.lisp:1050`) — caller-scoped
    /// character end-position used as the [`SuffixMapTemp`] lookup
    /// key. See [`crate::dict::_star_suffix_next_end_star_`]. Rebound
    /// at `dict.lisp:1091`, `dict.lisp:1852`, and `find-word-suffix`'s
    /// recursion at `dict-grammar.lisp:706`. `None` outside those
    /// scopes matches the upstream `(defvar … nil)` initform.
    ///
    /// Signed because `find-word-suffix`'s recursion subtracts
    /// `(length suffix)` off the current binding and can pass below
    /// zero — `usize` would panic on underflow, CL fixnums hold it
    /// without complaint. The map lookup converts via
    /// `usize::try_from(end).ok()` and drops negative-binding rows the
    /// same way `gethash` does.
    pub suffix_next_end: Option<i32>,

    /// Upstream `*split-map*` (`dict-split.lisp:5`) — selector for the
    /// active split-table binding. Rebound at `dict-split.lisp:786`
    /// inside `get-segsplit`.
    pub split_map: SplitMapKind,
}

impl std::ops::Deref for KaniranContext {
    type Target = KaniranShared;
    fn deref(&self) -> &Self::Target {
        &self.shared
    }
}

impl KaniranContext {
    /// Wrap process-lifetime [`KaniranShared`] state into a fresh
    /// context with every per-call binding at its default.
    fn from_shared(shared: KaniranShared) -> Self {
        Self {
            shared: Arc::new(shared),
            disable_hints: false,
            substring_hash: None,
            suffix_map_temp: None,
            suffix_next_end: None,
            split_map: SplitMapKind::Default,
        }
    }

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

    /// `(let ((*suffix-map-temp* v)) …)` — return a sibling context
    /// with the per-call-tree suffix-candidate cache set (or cleared
    /// with `None` per the upstream `(let ((*suffix-map-temp* nil)) …)`
    /// rebinds at `dict-grammar.lisp:442`, `:501`, `:555`).
    pub fn with_suffix_map_temp(&self, v: Option<Arc<SuffixMapTemp>>) -> Self {
        Self { suffix_map_temp: v, ..self.clone() }
    }

    /// `(let ((*suffix-next-end* v)) …)` — return a sibling context
    /// with the [`SuffixMapTemp`] lookup-key end-position set (or
    /// cleared with `None`; the `find-word-suffix` recursion at
    /// `dict-grammar.lisp:706` decrements through `nil` propagation).
    pub fn with_suffix_next_end(&self, v: Option<i32>) -> Self {
        Self { suffix_next_end: v, ..self.clone() }
    }

    /// `(let ((*split-map* *segsplit-map*)) …)` (`dict-split.lisp:786`).
    pub fn with_segsplit_map(&self) -> Self {
        Self { split_map: SplitMapKind::SegSplit, ..self.clone() }
    }

    /// True when the runtime store is the rkyv snapshot backend
    /// (`DATABASE_URL=memory://...`). Always true now that Postgres is
    /// gone; kept so callers that branch on it compile unchanged.
    pub fn is_rkyv(&self) -> bool {
        #[cfg(feature = "rkyv")]
        {
            matches!(self.store, KaniStore::Rkyv(_))
        }
        #[cfg(not(feature = "rkyv"))]
        {
            false
        }
    }
}

impl KaniranContext {
    /// Load the rkyv snapshot named by `url` and run every cache
    /// populator before returning. Only `memory://<path>` URLs are
    /// supported (feature `rkyv` required).
    pub fn from_url(url: &str) -> Result<Arc<Self>, Error> {
        let BuiltBackend {
            store,
            #[cfg(feature = "postgres")]
            pool,
        } = build_backend(url)?;
        let no_conj_data = Arc::new(build_no_conj_data(&store)?);
        let is_arch = Arc::new(build_is_arch(&store)?);
        let reading_cache = Arc::new(new_reading_cache());
        // The counter / suffix populators take a &KaniranContext (they
        // query the store through it). The cache fields live behind an
        // immutable Arc<KaniranShared>, so instead of swapping maps into
        // a mutable ctx, run each populator against a partial context,
        // then assemble the final shared state. Startup only.
        let counter_cache = {
            let partial = Self::from_shared(KaniranShared {
                #[cfg(feature = "postgres")]
                pool: pool.clone(),
                store: store.clone(),
                no_conj_data: no_conj_data.clone(),
                is_arch: is_arch.clone(),
                counter_cache: Arc::new(CounterCache::new()),
                suffix_cache: Arc::new(SuffixCache::new()),
                suffix_class: Arc::new(SuffixClass::new()),
                reading_cache: reading_cache.clone(),
            });
            Arc::new(build_counter_cache(&partial)?)
        };
        // build_suffix_caches reads the now-populated counter cache.
        let (suffix_cache, suffix_class) = {
            let partial = Self::from_shared(KaniranShared {
                #[cfg(feature = "postgres")]
                pool: pool.clone(),
                store: store.clone(),
                no_conj_data: no_conj_data.clone(),
                is_arch: is_arch.clone(),
                counter_cache: counter_cache.clone(),
                suffix_cache: Arc::new(SuffixCache::new()),
                suffix_class: Arc::new(SuffixClass::new()),
                reading_cache: reading_cache.clone(),
            });
            build_suffix_caches(&partial)?
        };
        Ok(Arc::new(Self::from_shared(KaniranShared {
            #[cfg(feature = "postgres")]
            pool: pool.clone(),
            store,
            no_conj_data,
            is_arch,
            counter_cache,
            suffix_cache: Arc::new(suffix_cache),
            suffix_class: Arc::new(suffix_class),
            reading_cache,
        })))
    }

    /// Read the snapshot URL via [`config::Config`] (file + env layered)
    /// and build the context.
    pub fn from_env() -> Result<Arc<Self>, Error> {
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
        Self::from_url(&url)
    }
}

/// Backend plus the optional Postgres pool produced alongside it.
struct BuiltBackend {
    store: KaniStore,
    #[cfg(feature = "postgres")]
    pool: Option<sqlx::PgPool>,
}

/// Load the backend for [`KaniranContext::from_url`]. Supports
/// `memory://<path>` (rkyv snapshot) and, under the `postgres` feature,
/// `postgres://` / `postgresql://` connection URLs.
fn build_backend(url: &str) -> Result<BuiltBackend, Error> {
    if let Some(path) = url.strip_prefix("memory://") {
        #[cfg(not(feature = "rkyv"))]
        {
            let _ = path;
            return Err(Error::Snapshot(
                "memory:// URL requires the `rkyv` feature to be enabled".into(),
            ));
        }
        #[cfg(feature = "rkyv")]
        {
            let backend = crate::conn::kani_rkyv_backend::KaniRkyvBackend::from_file(
                std::path::Path::new(path),
            )
            .map_err(Error::Snapshot)?;
            return Ok(BuiltBackend {
                store: KaniStore::Rkyv(backend),
                #[cfg(feature = "postgres")]
                pool: None,
            });
        }
    }
    #[cfg(feature = "postgres")]
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        let rt = std::sync::Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| Error::Snapshot(format!("tokio runtime: {e}")))?,
        );
        let pool = rt
            .block_on(
                sqlx::postgres::PgPoolOptions::new()
                    .max_connections(25)
                    .acquire_timeout(std::time::Duration::from_secs(10))
                    .connect(url),
            )
            .map_err(|e| Error::Snapshot(format!("postgres connect: {e}")))?;
        let backend =
            crate::conn::kani_postgres_backend::KaniPostgresBackend::new(pool.clone(), rt);
        return Ok(BuiltBackend {
            store: KaniStore::Postgres(backend),
            pool: Some(pool),
        });
    }
    Err(Error::Snapshot(format!(
        "only memory://<archive> URLs are supported after the Postgres backend removal; got `{url}`"
    )))
}
