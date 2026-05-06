//! Port of `ichiran/dict:*no-conj-data*` (`dict.lisp:329`).
//!
//! Per-seq marker for JMdict entries that have no rows in the
//! `conjugation` table — calculated negatively because the no-conj
//! set is much smaller than the conjugatable set and is more robust
//! when new conjugations are added (per upstream comment). Used as a
//! set: lookups go through [`super::no_conj_data::no_conj_data`],
//! which Lisp implements as
//! `(nth-value 1 (gethash seq (ensure :no-conj-data)))` — only key
//! presence matters.
//!
//! Stored as `HashMap<i32, bool>` (with `true` values) to mirror the
//! Lisp `(setf (gethash seq cache) t)` shape. Test via
//! `cache.contains_key(&seq)`.
//!
//! ## Lifecycle
//!
//! `OnceLock`-backed (memory `feedback_no_oncelock` retracted: lazy
//! locks are acceptable as interim until [`KaniranContext::Inner`]
//! lands). [`init_no_conj_data_cache`] runs the upstream
//! `defcache :no-conj-data` body — one SELECT against `entry LEFT
//! JOIN conjugation` — and locks the result behind the [`OnceLock`].
//! Idempotent: subsequent calls are no-ops, so the audit driver and
//! production drivers may both call it freely at startup.
//! [`no_conj_data_cache`] returns [`None`] until init runs, then
//! [`Some(&map)`] thereafter.
//!
//! ## Per-`KaniranContext` ownership (deferred)
//!
//! The cache is process-global today; the `Inner` refactor (memory
//! `project_kaniran_context_caches`) will move it onto a per-context
//! [`OnceCell`] field. Until then, multi-database use cases see one
//! shared cache populated by whichever context's
//! [`init_no_conj_data_cache`] ran first — fine for single-DB
//! production, audit, and tests.
//!
//! [`KaniranContext::Inner`]: super::super::conn::kani_context::KaniranContext

use crate::conn::kani_context::KaniranContext;
use std::collections::HashMap;
use std::sync::OnceLock;

static MAP: OnceLock<HashMap<i32, bool>> = OnceLock::new();

/// Borrow the no-conjugation-data seq registry. Returns [`None`]
/// until [`init_no_conj_data_cache`] has been awaited; afterward
/// returns the populated map for the lifetime of the process.
///
/// Named with the `_cache` suffix because the bare name
/// `no_conj_data` is reserved for the predicate function port at
/// `dict.lisp:339` (`super::no_conj_data::no_conj_data`).
pub fn no_conj_data_cache() -> Option<&'static HashMap<i32, bool>> {
    MAP.get()
}

/// Run the upstream `defcache :no-conj-data` body once: query
/// `entry LEFT JOIN conjugation` for seqs whose `conjugation.seq` is
/// NULL (i.e. entries with no conjugation rows), build a HashMap
/// keyed by seq, and lock it behind [`MAP`]. Idempotent — second and
/// later calls return [`Ok`] immediately.
pub async fn init_no_conj_data_cache(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    if MAP.get().is_some() {
        return Ok(());
    }
    let seqs: Vec<i32> = sqlx::query_scalar(
        "SELECT entry.seq FROM entry \
         LEFT JOIN conjugation c ON entry.seq = c.seq \
         WHERE c.seq IS NULL",
    )
    .fetch_all(&ctx.pool)
    .await?;
    let map: HashMap<i32, bool> = seqs.into_iter().map(|s| (s, true)).collect();
    // OnceLock::set returns Err if a concurrent caller won the race; their
    // map carries the same data (deterministic SQL), so discard the dup.
    let _ = MAP.set(map);
    Ok(())
}
