//! Port of `ichiran/numbers:num-sandhi` (`numbers.lisp:82-112`).
//!
//! Apply euphonic sound changes between two adjacent number kana
//! groups. Geminates the trailing mora of `s1` (e.g. `いち → いっ`) and
//! voices the leading mora of `s2` (e.g. `せん → ぜん`, `ひゃく →
//! びゃく`) when the `(c1, v1, c2, v2)` quadruple matches one of the
//! upstream `defmethod` specializers; otherwise the strings concatenate
//! unchanged.
//!
//! Diverges from the Lisp on one axis:
//!
//! - **Multimethod → match.** The Lisp uses six `defmethod` clauses on
//!   `(eql :jd N)` / `(eql :p N)` specializers plus a default that
//!   just concatenates. Rust collapses to a single `match` over
//!   `(c1, v1, c2, v2)`. The dispatch table is identical; the `_ →
//!   plain concat` arm covers the default method.
//!
//! Otherwise the 6-arg surface is preserved verbatim. `c1` and `v1`
//! are `Option`-typed because the Lisp permits passing `nil` for the
//! "no previous group" startup state (e.g. from `group-to-kana`'s
//! initial `last-class = nil`); the Lisp dispatch falls through to the
//! default method on `nil`, which the `_` arm replicates.

use crate::characters::voicing::geminate;
use crate::characters::voicing::{rendaku, Voicing};

use super::kani_num_class::NumClass;

pub fn num_sandhi(
    c1: Option<NumClass>,
    v1: Option<u8>,
    c2: NumClass,
    v2: u8,
    s1: &str,
    s2: &str,
) -> String {
    use NumClass::*;
    let mut s1_buf = s1.to_string();
    let mut s2_buf = s2.to_string();
    match (c1, v1, c2, v2) {
        (Some(Jd), Some(1), P, v) if matches!(v, 3 | 12 | 16) => {
            geminate(&mut s1_buf);
        }
        (Some(Jd), Some(3), P, v) if matches!(v, 2 | 3) => {
            rendaku(&mut s2_buf, Voicing::Dakuten);
        }
        (Some(Jd), Some(6), P, 2) => {
            geminate(&mut s1_buf);
            rendaku(&mut s2_buf, Voicing::Handakuten);
        }
        (Some(Jd), Some(6), P, 16) => {
            geminate(&mut s1_buf);
        }
        (Some(Jd), Some(8), P, 2) => {
            geminate(&mut s1_buf);
            rendaku(&mut s2_buf, Voicing::Handakuten);
        }
        (Some(Jd), Some(8), P, v) if matches!(v, 3 | 12 | 16) => {
            geminate(&mut s1_buf);
        }
        (Some(P), Some(1), P, v) if matches!(v, 12 | 16) => {
            geminate(&mut s1_buf);
        }
        (Some(P), Some(2), P, 16) => {
            geminate(&mut s1_buf);
        }
        _ => {}
    }
    format!("{s1_buf}{s2_buf}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use NumClass::*;

    #[test]
    fn no_prev_just_concatenates() {
        assert_eq!(num_sandhi(None, None, Jd, 5, "", "ご"), "ご");
    }

    #[test]
    fn ichi_sen_geminates_s1() {
        // (Jd 1) + (P 3) → 1+千 → s1 "いち" → "いっ", s2 "せん" unchanged.
        assert_eq!(
            num_sandhi(Some(Jd), Some(1), P, 3, "いち", "せん"),
            "いっせん"
        );
    }

    #[test]
    fn san_byaku_voices_s2() {
        // (Jd 3) + (P 2) → 3+百 → s2 "ひゃく" → "びゃく".
        assert_eq!(
            num_sandhi(Some(Jd), Some(3), P, 2, "さん", "ひゃく"),
            "さんびゃく"
        );
    }

    #[test]
    fn roku_pyaku_geminates_and_handakuten() {
        // (Jd 6) + (P 2) → 6+百 → "ろく" → "ろっ", "ひゃく" → "ぴゃく".
        assert_eq!(
            num_sandhi(Some(Jd), Some(6), P, 2, "ろく", "ひゃく"),
            "ろっぴゃく"
        );
    }

    #[test]
    fn unmatched_pair_just_concatenates() {
        assert_eq!(
            num_sandhi(Some(Jd), Some(2), P, 2, "に", "ひゃく"),
            "にひゃく"
        );
    }
}
