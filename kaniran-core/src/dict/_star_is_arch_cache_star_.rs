//! Port of `ichiran/dict:*is-arch-cache*` (`dict.lisp:745`).
//!
//! Per-seq marker for entries whose every sense is tagged
//! `arch` / `obsc` / `rare` (or whose conjugation roots are such
//! entries). Used as a set: lookups go through `is-arch (seq)`, which
//! Lisp implements as `(nth-value 1 (gethash seq (ensure :is-arch)))`
//! — the second value of `gethash` is the found-p flag, so only key
//! presence matters.
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
//! lazily by the `defcache :is-arch` body, which runs two SQL queries
//! (a misc-tag GROUP BY having every-not-null check, then a
//! conjugation-root union). Long-term home is a per-`Inner`
//! `OnceCell` field on `KaniranContext` (memory
//! `project_kaniran_context_caches`).
//!
//! The registry is currently empty — the populator is unported
//! (depends on the JMdict-schema decision and the `sense-prop` /
//! `conjugation` query layer).

use std::collections::HashMap;
use std::sync::OnceLock;

/// Borrow the archaic-seq registry.
pub fn is_arch_cache() -> &'static HashMap<i32, bool> {
    static MAP: OnceLock<HashMap<i32, bool>> = OnceLock::new();
    MAP.get_or_init(HashMap::new)
}
