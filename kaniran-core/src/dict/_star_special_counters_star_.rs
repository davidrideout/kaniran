//! Port of `ichiran/dict:*special-counters*` (`dict-counters.lisp:211`).
//!
//! Per-seq counter-args registry. Lisp init form is
//! `(make-hash-table)` — initialized empty; ~50
//! `def-special-counter` macro callsites scattered through
//! `dict-counters.lisp` populate it at file load. Each callsite
//! registers a function that, given the readings list for a JMdict
//! seq, yields the constructor arg sets that the counter cache
//! populator stores under that seq's text keys.
//!
//! ## Value-fn shape
//!
//! Each registered function takes the readings list (kanji + kana
//! rows for one seq, flattened) and returns a vector of
//! `(text_key, prototype Counter)` pairs. The cache populator stores
//! each pair under `text_key`; `find-counter` later clones the
//! prototype, fills in `number_text` / `number`, and runs `verify`.
//!
//! Multi-text args (Lisp `:text '("丁" "目")`) emit one entry per
//! text variant, with each entry's `source` resolved to the matching
//! reading row. The Lisp builds this lazily via a closure on the
//! `:source` initarg; in Rust we resolve eagerly inside the
//! registered function — plain `fn` pointers, no captures needed.
//!
//! ## Storage
//!
//! `OnceLock`-backed for now per memory `feedback_no_oncelock`.
//! Long-term home is `KaniranContext::Inner` (memory
//! `project_kaniran_context_caches`); migration is a mechanical
//! refactor when `Inner` lands.
//!
//! The registry is currently empty — no `def-special-counter`
//! callsites have been ported. Each one becomes a top-level function
//! and an entry in the `register` block inside [`special_counters`].

use crate::dict::counter_text_class::{Counter, CounterSource};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Registered constructor for one JMdict seq. Returns
/// `(text_key, prototype_counter)` pairs to land in the cache.
pub type SpecialCounterFn = fn(readings: &[CounterSource]) -> Vec<(String, Counter)>;

/// Borrow the per-seq special-counter registry.
pub fn special_counters() -> &'static HashMap<i32, SpecialCounterFn> {
    static MAP: OnceLock<HashMap<i32, SpecialCounterFn>> = OnceLock::new();
    MAP.get_or_init(HashMap::new)
}
