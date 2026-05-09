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
//! ## Divergence
//!
//! The Lisp `defparameter` is dynamically rebound by `let`. Rust has no
//! native dynamic binding, so this is modelled as a thread-local
//! `Option` plus a scoped RAII guard ([`with_substring_hash`]). Same
//! observable effect on a single thread (the rebinding is visible to
//! inner calls, restored on guard drop), matching the
//! [`super::_star_disable_hints_star_`] precedent.
//!
//! ## TODO: tokio task migration
//!
//! [`super::find_word::find_word`] is async (DB-backed). Once
//! `find-word-full` and `find-substring-words` (waves 326 / 325) are
//! ported — both async — a tokio multi-thread runtime can move the
//! task between worker threads at every `.await`, dropping the
//! thread-local binding on the receiving worker. Two safer shapes for
//! that point:
//!
//! 1. Switch this module to `tokio::task_local!` so the binding
//!    follows the task across worker threads.
//! 2. Pass the cache as an explicit `Option<&SubstringHash>`
//!    parameter threaded through `find-word-full` → `find-word`.
//!
//! Today the only reader is [`super::find_word::find_word`]; with no
//! producer wired up yet, the thread-local resolves to `None` on
//! every call and the cache check is a no-op. Whichever option lands
//! later, `find_word`'s read site is a one-line update.

use crate::dict::find_word::FindWordRows;
use std::cell::RefCell;
use std::collections::HashMap;

/// Map from a substring of an input string to the `kana_text` /
/// `kanji_text` rows pre-fetched for it by `find-substring-words`.
/// Per-key uniformity (all rows from one table) is enforced by the
/// populator's kana-vs-kanji key split (`dict.lisp:511`).
pub type SubstringHash = HashMap<String, FindWordRows>;

thread_local! {
    static SUBSTRING_HASH: RefCell<Option<SubstringHash>> = RefCell::new(None);
}

/// Read the cached rows for `key`, if any. Returns a clone so the
/// caller doesn't borrow the thread-local across `.await` points.
pub fn substring_hash_get(key: &str) -> Option<FindWordRows> {
    SUBSTRING_HASH.with(|c| c.borrow().as_ref().and_then(|h| h.get(key).cloned()))
}

pub struct SubstringHashGuard {
    prev: Option<SubstringHash>,
}

impl Drop for SubstringHashGuard {
    fn drop(&mut self) {
        let prev = self.prev.take();
        SUBSTRING_HASH.with(|c| *c.borrow_mut() = prev);
    }
}

/// Bind `*substring-hash*` to `value` for the lifetime of the
/// returned guard. Mirrors a Lisp
/// `(let ((*substring-hash* ...)) ...)` form.
pub fn with_substring_hash(value: Option<SubstringHash>) -> SubstringHashGuard {
    let prev = SUBSTRING_HASH.with(|c| c.replace(value));
    SubstringHashGuard { prev }
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
    fn default_is_none() {
        assert!(substring_hash_get("foo").is_none());
    }

    #[test]
    fn guard_rebinds_and_restores() {
        let mut h: SubstringHash = HashMap::new();
        h.insert("ぁ".into(), FindWordRows::Kana(vec![kana_row(1, "ぁ")]));
        {
            let _g = with_substring_hash(Some(h));
            match substring_hash_get("ぁ") {
                Some(FindWordRows::Kana(v)) => assert_eq!(v.len(), 1),
                other => panic!("expected Kana(1), got {:?}", other),
            }
        }
        assert!(substring_hash_get("ぁ").is_none());
    }
}
