//! Port of `ichiran/dict:*substring-hash*` (`dict.lisp:487`).
//!
//! Per-call-tree cache of pre-fetched `kana_text` / `kanji_text` rows
//! keyed by their `text` column, letting nested `find-word` calls skip
//! the database round-trip for keys already present.

use crate::dict::find_word::FindWordRows;
use std::collections::HashMap;

/// Map from a substring of an input string to the `kana_text` /
/// `kanji_text` rows pre-fetched for it by `find-substring-words`.
/// Per-key uniformity (all rows from one table) is enforced by the
/// populator's kana-vs-kanji key split (`dict.lisp:511`).
pub type SubstringHash = HashMap<String, FindWordRows>;
