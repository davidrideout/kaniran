//! Port of `ichiran/dict:*substring-hash*` (`dict.lisp:487`).
//!
//! Per-call-tree cache of pre-fetched `kana_text` / `kanji_text` rows
//! keyed by their `text` column. Upstream is a `defparameter` whose
//! default value is `nil`; the only producer is `find-substring-words`,
//! which dynamically rebinds it inside `find-word-full`'s caller via
//! `let`. While bound, every nested
//! [`super::find_word::find_word`] call short-circuits the database
//! round-trip for keys present in the hash (root-only excluded —
//! upstream always re-queries with the JOIN against `entry`).
//!
//! ## Rust shape — ctx slot, not a dynamic binding
//!
//! Rust has no native dynamic binding. The current port carries the
//! value on [`crate::conn::kani_context::KaniranContext`] as an
//! `Option<Arc<SubstringHash>>` field; rebinding is
//! [`crate::conn::kani_context::KaniranContext::with_substring_hash`]
//! returning a sibling ctx. Survives crossing rayon / `tokio::spawn`
//! boundaries because the rebound ctx is propagated by value /
//! reference, not by a thread-local or task-local side channel
//! (both of which would silently drop the binding at parallel
//! worker handoff).
//!
//! Today no producer wires up the cache, so reads always observe
//! `None` and the find-word short-circuit is dead at runtime. The
//! wire-up lands when wave 326 (`find-word-full`) and wave 325
//! (`find-substring-words`) port — both will populate a
//! `SubstringHash`, wrap it in [`std::sync::Arc`], and rebind the
//! ctx via `with_substring_hash` before invoking nested
//! [`super::find_word::find_word`] calls.

use crate::dict::find_word::FindWordRows;
use std::collections::HashMap;

/// Map from a substring of an input string to the `kana_text` /
/// `kanji_text` rows pre-fetched for it by `find-substring-words`.
/// Per-key uniformity (all rows from one table) is enforced by the
/// populator's kana-vs-kanji key split (`dict.lisp:511`).
pub type SubstringHash = HashMap<String, FindWordRows>;
