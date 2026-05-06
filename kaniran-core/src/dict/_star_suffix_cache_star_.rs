//! Port of `ichiran/dict:*suffix-cache*` (`dict-grammar.lisp:5`).
//!
//! Per-text registry of grammatical-suffix matches. Keys are the
//! surface text of a suffix (Lisp `equal` test); each value is the
//! list of `(class_keyword, optional kana-form row)` matches the
//! populator has loaded for that text. A length-1 vec is the single-
//! match case (Lisp `(list key kf)`); longer vecs are the join case
//! (Lisp `(cons new old)` / `(list new old)` chains assembled inside
//! `update-suffix-cache`).
//!
//! ## Value-list shape
//!
//! - `class_keyword` is the Lisp suffix class (`:teiru`, `:te`,
//!   `:iru`, `:ha`, `:chau`, …). Carried as `String` for now — the
//!   set is open relative to what the consumers/populator have
//!   exposed; collapses to a closed enum once
//!   [`init_suffixes_thread`] (wave ~169) and the
//!   `def-simple-suffix` callsites are ported.
//! - The optional kana-form row references a [`KanaText`] DAO row.
//!   `None` corresponds to the `load-abbr` callsite shape `(list key
//!   nil)` (abbreviated forms with no source row).
//!
//! ## Storage
//!
//! `OnceLock`-backed for now per memory `feedback_no_oncelock`. The
//! upstream is a `def-conn-var` (per-connection-rebound global)
//! initialised to `nil` and populated on first connection by
//! `init-suffixes-thread`. The Rust equivalent is a per-`Inner`
//! `OnceCell` field; migration is mechanical when `KaniranContext::Inner`
//! lands (memory `project_kaniran_context_caches`).
//!
//! The registry is currently empty — the populator has not been ported.

use crate::dict::kana_text_dao::KanaText;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Borrow the per-text suffix registry.
pub fn suffix_cache() -> &'static HashMap<String, Vec<(String, Option<KanaText>)>> {
    static MAP: OnceLock<HashMap<String, Vec<(String, Option<KanaText>)>>> = OnceLock::new();
    MAP.get_or_init(HashMap::new)
}
