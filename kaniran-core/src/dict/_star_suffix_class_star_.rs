//! Port of `ichiran/dict:*suffix-class*` (`dict-grammar.lisp:6`).
//!
//! Per-seq registry of which suffix class a JMdict entry belongs to —
//! "seq → class" per the upstream comment. Keys are JMdict seqs (Lisp
//! `eql` test); values are the class keyword carried by the entry's
//! suffix-cache rows (`:teiru`, `:te`, `:iru`, `:ha`, …).
//!
//! Consumed by `find-word-with-suffix` /
//! `find-word-suffix` / `get-suffix-description` to resolve a word's
//! class without re-scanning the suffix table, and populated by
//! `init-suffixes-thread`'s `load-kf` helper (`(setf (gethash (seq kf)
//! *suffix-class*) (or class key))`).
//!
//! Class values are carried as `String` for now — same rationale as
//! [`crate::dict::_star_suffix_cache_star_`]: the keyword set is open
//! until the populator and `def-simple-suffix` callsites are ported,
//! at which point this collapses to a closed enum.
//!
//! ## Storage
//!
//! `OnceLock`-backed for now per memory `feedback_no_oncelock`. The
//! upstream is a `def-conn-var` initialised to `nil` and populated
//! lazily by `init-suffixes-thread`. The long-term home is a per-
//! `Inner` `OnceCell` field on `KaniranContext` (memory
//! `project_kaniran_context_caches`).
//!
//! The registry is currently empty — the populator has not been ported.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Borrow the per-seq suffix-class registry.
pub fn suffix_class() -> &'static HashMap<i32, String> {
    static MAP: OnceLock<HashMap<i32, String>> = OnceLock::new();
    MAP.get_or_init(HashMap::new)
}
