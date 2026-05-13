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
//! ## Rust shape — explicit parameter, not a dynamic binding
//!
//! Rust has no native dynamic binding. An earlier port used a
//! [`thread_local!`] + RAII guard; the binding wouldn't survive
//! `.await` points on the multi-threaded tokio runtime (the task can
//! resume on a different worker, dropping the binding silently). The
//! current port threads `Option<&SubstringHash>` as an explicit
//! parameter through [`super::find_word::find_word`] — same pattern
//! as `&KaniranContext` replaces `*connection*` per
//! [`crate::conn::kani_context`], and `disable_hints: bool` replaces
//! `*disable-hints*` in the hint cluster.
//!
//! Today no producer wires up the cache, so the only callsite passes
//! `None` and the path is dead at runtime. The wire-up lands when
//! wave 326 (`find-word-full`) and wave 325 (`find-substring-words`)
//! port — both already async, both will receive `&SubstringHash` and
//! forward it into nested `find_word` calls.

use crate::dict::find_word::FindWordRows;
use std::collections::HashMap;

/// Map from a substring of an input string to the `kana_text` /
/// `kanji_text` rows pre-fetched for it by `find-substring-words`.
/// Per-key uniformity (all rows from one table) is enforced by the
/// populator's kana-vs-kanji key split (`dict.lisp:511`).
pub type SubstringHash = HashMap<String, FindWordRows>;

/// Look up `key` in the cache. Returns a clone so the caller doesn't
/// borrow the cache across `.await` points. `None` covers both the
/// "cache absent" (`None` argument) and "key not present" cases —
/// mirroring upstream `(when *substring-hash* (gethash key
/// *substring-hash*))` semantics.
pub fn substring_hash_get(
    cache: Option<&SubstringHash>,
    key: &str,
) -> Option<FindWordRows> {
    cache.and_then(|h| h.get(key).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::find_word::FindWordRows;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::simple_text_class::SimpleText;

    fn kana_row(seq: i32, text: &str) -> KanaText {
        KanaText {
            id: 0, seq, text: text.into(), ord: 0,
            common: None, common_tags: String::new(), conjugate_p: true,
            nokanji: false, best_kanji: None, state: SimpleText::default(),
        }
    }

    #[test]
    fn absent_cache_returns_none() {
        assert!(substring_hash_get(None, "foo").is_none());
    }

    #[test]
    fn present_cache_with_missing_key_returns_none() {
        let h: SubstringHash = HashMap::new();
        assert!(substring_hash_get(Some(&h), "foo").is_none());
    }

    #[test]
    fn present_cache_with_hit_returns_clone() {
        let mut h: SubstringHash = HashMap::new();
        h.insert("ぁ".into(), FindWordRows::Kana(vec![kana_row(1, "ぁ")]));
        match substring_hash_get(Some(&h), "ぁ") {
            Some(FindWordRows::Kana(v)) => assert_eq!(v.len(), 1),
            other => panic!("expected Kana(1), got {:?}", other),
        }
    }
}
