//! Rust-only sidecar: the port-wide context, holding the Postgres
//! connection pool and every populated cache. Replaces upstream's
//! `*connection*`, the `cache`-class registry, and the per-connection
//! variable cache; multi-DB use means constructing another
//! `KaniranContext`.

use crate::conn::_star_connection_env_var_star_::DATABASE_URL;
use crate::conn::get_ichiran_connection_env::get_ichiran_connection_env;
use crate::conn::kani_backend::KaniStore;
use crate::conn::kani_postgres_backend::KaniPostgresBackend;
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
    #[error("rkyv snapshot: {0}")]
    Snapshot(String),
}

#[derive(Clone)]
pub struct KaniranContext {
    pub pool: PgPool,
    /// Dictionary lookup backend. All runtime (lookup-serving)
    /// queries go through here; `pool` remains for build-time code
    /// that writes the database (`dict/load`, `dict/errata`, kanjidic
    /// loaders). Postgres by default. With the `rkyv` feature, a
    /// `DATABASE_URL=memory://<archive path>` selects the
    /// memory-mapped snapshot backend instead; in that mode `pool`
    /// holds a lazy placeholder that never connects — build-time
    /// tools require a real `postgres://` URL.
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

    /// Upstream `*disable-hints*` (`dict.lisp:78`) — recursion guard
    /// for the `simple-text :around` method on
    /// [`crate::dict::accessors::get_kana`]. Rebound to `true` at two
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
    /// (`DATABASE_URL=memory://...`). Audit harness uses this to pick
    /// CPU-parallel vs pool-driven concurrency without re-reading env.
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
    /// Connect the pool and run every cache populator before returning.
    /// Dispatches on URL scheme:
    /// - `postgres://...` → Postgres backend (pool cap 25: small enough
    ///   that two audit processes plus a bystander session fit inside
    ///   Postgres' default `max_connections = 100`, large enough to feed
    ///   the audit runners' default task concurrency).
    /// - `memory://<path>` → rkyv snapshot loaded from `<path>` (feature
    ///   `rkyv` required); the `pool` field holds a `connect_lazy`
    ///   placeholder that never connects, so any build-time code path
    ///   that uses `&ctx.pool` fails on first use.
    pub async fn from_url(url: &str) -> Result<Arc<Self>, Error> {
        let (pool, store) = build_backend(url).await?;
        // The DB-derived caches are identical across every context built
        // in a single test process and dominate build time (~5s of table
        // scans each). During the crate's own tests, build them once and
        // share the Arcs; each context still gets its own runtime-bound
        // pool (a shared pool's connections die when a per-test runtime is
        // torn down) and a fresh reading-cache. Production builds fresh.
        #[cfg(test)]
        {
            let (no_conj_data, is_arch, counter_cache, suffix_cache, suffix_class) =
                test_support::shared_caches(&pool).await?;
            return Ok(Arc::new(Self {
                pool,
                store,
                no_conj_data,
                is_arch,
                counter_cache,
                suffix_cache,
                suffix_class,
                reading_cache: Arc::new(new_reading_cache()),
                disable_hints: false,
                substring_hash: None,
                suffix_map_temp: None,
                suffix_next_end: None,
                split_map: SplitMapKind::Default,
            }));
        }
        #[cfg(not(test))]
        {
            let no_conj_data = Arc::new(build_no_conj_data(&store).await?);
            let is_arch = Arc::new(build_is_arch(&store).await?);
            // counter_cache + suffix_cache populators call DB-touching fns
            // that take &KaniranContext — build a partial ctx first, then
            // swap the populated maps in.
            let mut ctx = Self {
                pool,
                store,
                no_conj_data,
                is_arch,
                counter_cache: Arc::new(CounterCache::new()),
                suffix_cache: Arc::new(SuffixCache::new()),
                suffix_class: Arc::new(SuffixClass::new()),
                reading_cache: Arc::new(new_reading_cache()),
                disable_hints: false,
                substring_hash: None,
                suffix_map_temp: None,
                suffix_next_end: None,
                split_map: SplitMapKind::Default,
            };
            ctx.counter_cache = Arc::new(build_counter_cache(&ctx).await?);
            let (suffix_cache, suffix_class) = build_suffix_caches(&ctx).await?;
            ctx.suffix_cache = Arc::new(suffix_cache);
            ctx.suffix_class = Arc::new(suffix_class);
            Ok(Arc::new(ctx))
        }
    }

    /// Connect the pool and return a context with every cache empty.
    /// Rust-only sidecar — no Lisp counterpart. The cache populators
    /// touch JMdict tables that may be absent or empty (e.g. on a fresh
    /// schema that the kanjidic loaders are about to populate); this
    /// constructor skips them so the e2e tooling can run against such
    /// shells. Callers must not invoke functions that read these caches
    /// (`find-word` / `get-counter-ids` / suffix dispatch / …) — they
    /// would silently see empty maps.
    ///
    /// `memory://` URLs are rejected: this constructor is for build-time
    /// tools that write the database and need a real Postgres pool.
    pub async fn pool_only_from_url(url: &str) -> Result<Arc<Self>, Error> {
        use std::collections::HashMap;

        if url.starts_with("memory://") {
            return Err(Error::Snapshot(
                "pool_only_from_url is for build-time tools and requires a postgres:// URL — \
                 memory:// (rkyv snapshot) is read-only".into(),
            ));
        }
        let pool = PgPoolOptions::new()
            .max_connections(60)
            .acquire_timeout(Duration::from_secs(10))
            .connect(url)
            .await
            .map_err(|e| {
                eprintln!("kaniran: failed to connect to database at `{url}`: {e}");
                Error::from(e)
            })?;
        Ok(Arc::new(Self {
            store: KaniStore::Postgres(KaniPostgresBackend::new(pool.clone())),
            pool,
            no_conj_data: Arc::new(HashSet::new()),
            is_arch: Arc::new(HashSet::new()),
            counter_cache: Arc::new(HashMap::new()),
            suffix_cache: Arc::new(HashMap::new()),
            suffix_class: Arc::new(HashMap::new()),
            reading_cache: Arc::new(new_reading_cache()),
            disable_hints: false,
            substring_hash: None,
            suffix_map_temp: None,
            suffix_next_end: None,
            split_map: SplitMapKind::Default,
        }))
    }

    /// Read a Postgres URL via [`config::Config`] (file + env layered)
    /// and build the context.
    pub async fn from_env() -> Result<Arc<Self>, Error> {
        // (the store backend is chosen inside from_url)
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

#[cfg(test)]
impl KaniranContext {
    /// Test-only constructor for DDL / schema-touching tests. Connects
    /// a pool but leaves every cache empty — no `build_no_conj_data` /
    /// `build_counter_cache` / etc. run, so it survives an empty
    /// schema where the production populators would fail.
    ///
    /// Refuses to run unless `KANIRAN_TEST_DATABASE_URL` is set and
    /// differs from `DATABASE_URL`. This guards `init_tables` (and any
    /// other DDL test) from ever wiping the production corpus.
    pub(crate) async fn pool_only_test_ctx() -> Self {
        use crate::dict::split::split_map::SplitMapKind;
        use std::collections::HashMap;

        let test_url = std::env::var("KANIRAN_TEST_DATABASE_URL").expect(
            "KANIRAN_TEST_DATABASE_URL must be set for #[ignore]d DDL tests \
             (e.g. postgres://localhost/kaniran_test)",
        );
        // Cross-check against the resolved production URL the same way
        // `from_env` resolves it (config file layered under env vars),
        // not just `std::env::var(DATABASE_URL)` — a kaniran.toml setting
        // would otherwise sneak past the guard.
        if let Ok(Some(prod_url)) = get_ichiran_connection_env() {
            assert_ne!(
                test_url, prod_url,
                "KANIRAN_TEST_DATABASE_URL must differ from the resolved \
                 production database URL — refusing to run DDL against production",
            );
        }
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&test_url)
            .await
            .expect("connect to kaniran_test");
        KaniranContext {
            store: KaniStore::Postgres(KaniPostgresBackend::new(pool.clone())),
            pool,
            no_conj_data: Arc::new(HashSet::new()),
            is_arch: Arc::new(HashSet::new()),
            counter_cache: Arc::new(HashMap::new()),
            suffix_cache: Arc::new(HashMap::new()),
            suffix_class: Arc::new(HashMap::new()),
            reading_cache: Arc::new(new_reading_cache()),
            disable_hints: false,
            substring_hash: None,
            suffix_map_temp: None,
            suffix_next_end: None,
            split_map: SplitMapKind::Default,
        }
    }
}

/// Connect/load the backend for [`KaniranContext::from_url`] based on
/// the URL scheme:
///
/// - `postgres://...` — connect a 25-conn pool and serve via
///   [`KaniPostgresBackend`].
/// - `memory://<path>` (feature `rkyv` only) — `mmap` the rkyv
///   snapshot at `<path>` and serve via [`KaniRkyvBackend`]. The
///   returned `pool` is a `connect_lazy` placeholder that never opens
///   a TCP connection; any code path that calls `&ctx.pool` for SQL
///   will fail on first use, which is intentional — build-time tools
///   (`kanjidic-load`, dumper, etc.) require a real `postgres://`
///   URL.
async fn build_backend(url: &str) -> Result<(PgPool, KaniStore), Error> {
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
            // Placeholder pool — connect_lazy validates URL syntax but
            // never opens a connection. The URL is bogus on purpose so
            // accidental use surfaces a recognizable host.
            let pool = PgPoolOptions::new()
                .max_connections(1)
                .connect_lazy("postgres://kaniran-memory-mode@localhost/_unused_in_memory_mode")
                .map_err(Error::from)?;
            return Ok((pool, KaniStore::Rkyv(backend)));
        }
    }

    let pool = PgPoolOptions::new()
        .max_connections(25)
        .acquire_timeout(Duration::from_secs(10))
        .connect(url)
        .await
        .map_err(|err| {
            eprintln!("kaniran: failed to connect to database at `{url}`: {err}");
            Error::from(err)
        })?;
    let store = KaniStore::Postgres(KaniPostgresBackend::new(pool.clone()));
    Ok((pool, store))
}

/// Shared cache builder for the crate's own test runs. The five
/// DB-derived caches are deterministic from database content, so they
/// are built once per test process and reused across every
/// `#[tokio::test]` context. The cache maps are plain runtime-independent
/// data — sharing them across the per-test tokio runtimes is sound,
/// unlike the `PgPool` whose connections are bound to the runtime that
/// opened them. Not compiled into production builds.
#[cfg(test)]
mod test_support {
    use super::*;
    use tokio::sync::OnceCell;

    type Caches = (
        Arc<HashSet<i32>>,
        Arc<HashSet<i32>>,
        Arc<CounterCache>,
        Arc<SuffixCache>,
        Arc<SuffixClass>,
    );

    static SHARED: OnceCell<Caches> = OnceCell::const_new();

    pub(super) async fn shared_caches(pool: &PgPool) -> Result<Caches, Error> {
        let caches = SHARED.get_or_try_init(|| build_once(pool)).await?;
        Ok(caches.clone())
    }

    /// Mirrors the cache-build sequence in
    /// [`KaniranContext::from_url`]'s production path, run against a
    /// throwaway context whose pool belongs to the first test that
    /// triggers the build. Only the resulting cache Arcs are retained.
    async fn build_once(pool: &PgPool) -> Result<Caches, Error> {
        let store = KaniStore::Postgres(KaniPostgresBackend::new(pool.clone()));
        let no_conj_data = Arc::new(build_no_conj_data(&store).await?);
        let is_arch = Arc::new(build_is_arch(&store).await?);
        let mut ctx = KaniranContext {
            pool: pool.clone(),
            store,
            no_conj_data: no_conj_data.clone(),
            is_arch: is_arch.clone(),
            counter_cache: Arc::new(CounterCache::new()),
            suffix_cache: Arc::new(SuffixCache::new()),
            suffix_class: Arc::new(SuffixClass::new()),
            reading_cache: Arc::new(new_reading_cache()),
            disable_hints: false,
            substring_hash: None,
            suffix_map_temp: None,
            suffix_next_end: None,
            split_map: SplitMapKind::Default,
        };
        let counter_cache = Arc::new(build_counter_cache(&ctx).await?);
        ctx.counter_cache = counter_cache.clone();
        let (suffix_cache, suffix_class) = build_suffix_caches(&ctx).await?;
        Ok((
            no_conj_data,
            is_arch,
            counter_cache,
            Arc::new(suffix_cache),
            Arc::new(suffix_class),
        ))
    }
}
