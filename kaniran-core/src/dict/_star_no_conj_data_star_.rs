//! Port of `ichiran/dict:*no-conj-data*` (`dict.lisp:329`).
//!
//! Per-seq marker for JMdict entries that have no rows in the
//! `conjugation` table — calculated negatively because the no-conj
//! set is much smaller than the conjugatable set and is more robust
//! when new conjugations are added (per upstream comment). Used as a
//! set: lookups go through `no-conj-data (seq)`, which Lisp
//! implements as `(nth-value 1 (gethash seq (ensure :no-conj-data)))`
//! — only key presence matters.
//!
//! Stored as `HashMap<i32, bool>` (with `true` values) to mirror the
//! Lisp shape literally — the upstream `(setf (gethash seq cache) t)`.
//! Callers should test via `cache.contains_key(&seq)`, the direct
//! analogue of Lisp's `nth-value 1 gethash`.
//!
//! ## Storage
//!
//! `OnceLock`-backed for now per memory `feedback_no_oncelock`. The
//! upstream is a `def-conn-var` initialised to `nil` and populated
//! lazily by the `defcache :no-conj-data` body, which queries
//! `entry LEFT JOIN conjugation` for seqs with no conjugation row
//! (sized for the ~200k entries in JMdict). Long-term home is a
//! per-`Inner` `OnceCell` field on `KaniranContext` (memory
//! `project_kaniran_context_caches`).
//!
//! The registry is currently empty — the populator is unported
//! (depends on the JMdict-schema decision and the `entry` /
//! `conjugation` query layer).

use std::collections::HashMap;
use std::sync::OnceLock;

/// Borrow the no-conjugation-data seq registry. Named with the
/// `_cache` suffix because the bare name `no_conj_data` is reserved
/// for the upcoming port of the Lisp function `no-conj-data`
/// (`dict.lisp:339`), which queries this registry.
pub fn no_conj_data_cache() -> &'static HashMap<i32, bool> {
    static MAP: OnceLock<HashMap<i32, bool>> = OnceLock::new();
    MAP.get_or_init(HashMap::new)
}
