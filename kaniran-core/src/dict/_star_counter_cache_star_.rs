//! Port of `ichiran/dict:*counter-cache*` (`dict-counters.lisp:221`).
//!
//! Per-text registry of counter-instance prototypes, keyed by the
//! counter's surface text (Lisp `equal` test). Values are the list of
//! [`Counter`] prototypes the populator has loaded for that text;
//! `find-counter` (unported) clones a prototype, fills in
//! `number_text` / `number`, and runs `verify`.
//!
//! ## Storage
//!
//! `OnceLock`-backed for now per memory `feedback_no_oncelock`. The
//! upstream is a `def-conn-var` initialised to `nil` and populated
//! lazily by the `defcache :counters` body — which queries the DB
//! (`get-counter-readings`) and then runs through every
//! `*special-counters*` callsite plus the default counter-text /
//! number-text fallback. Long-term home is a per-`Inner` `OnceCell`
//! field on `KaniranContext` (memory
//! `project_kaniran_context_caches`).
//!
//! The registry is currently empty — the populator is unported
//! (depends on `get-counter-readings`, the populated
//! `*special-counters*`, and the JMdict-schema decision).

use crate::dict::counter_text_class::Counter;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Borrow the per-text counter-prototype registry.
pub fn counter_cache() -> &'static HashMap<String, Vec<Counter>> {
    static MAP: OnceLock<HashMap<String, Vec<Counter>>> = OnceLock::new();
    MAP.get_or_init(HashMap::new)
}
