//! Port of `ichiran:*romaji-kana-next*` (`deromanize.lisp:21`).
//!
//! Set of every proper prefix of every romaji key in
//! [`*romaji-kana*`][super::_star_romaji_kana_star_::romaji_kana] —
//! the "could this grow into a longer key?" membership test consulted
//! by `romaji-next`. Lazily built once via [`has_successors`] over the
//! map's keys, mirroring the upstream `defparameter` evaluated after
//! `load-romaji-kana`.

use std::collections::HashSet;
use std::sync::OnceLock;

use super::_star_romaji_kana_star_::romaji_kana;
use super::has_successors::has_successors;

static ROMAJI_KANA_NEXT: OnceLock<HashSet<String>> = OnceLock::new();

pub fn romaji_kana_next() -> &'static HashSet<String> {
    ROMAJI_KANA_NEXT.get_or_init(|| {
        let keys: Vec<&str> = romaji_kana().keys().map(String::as_str).collect();
        has_successors(&keys)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL (.103): `(hash-table-count *romaji-kana-next*)` = 60.
    /// Membership spot-checks pin the proper-prefix contract: short
    /// prefixes of longer keys are present, full keys are not.
    #[test]
    fn builds_once() {
        let next = romaji_kana_next();
        assert_eq!(next.len(), 60);
        for present in ["k", "ky", "c", "ch", "b", "s", "sh"] {
            assert!(next.contains(present), "expected {present:?} present");
        }
        for absent in ["ka", "a", "zz"] {
            assert!(!next.contains(absent), "expected {absent:?} absent");
        }
    }
}
