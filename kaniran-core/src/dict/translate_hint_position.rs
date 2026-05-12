//! Port of `ichiran/dict:translate-hint-position` (`dict-split.lisp:882-897`).
//!
//! Walks `matched` — a heterogeneous alignment list whose elements
//! are either an equal substring or a `(pre, post)` differing pair —
//! and translates a character index `position` over the pre-image
//! axis into a character index on the post-image axis. Returns
//! `None` when `position` overshoots the alignment's total
//! pre-image length (the upstream `loop` falls through and returns
//! `nil`).
//!
//! The two diff-pair sub-cases mirror the upstream `cond` exactly:
//! - `position` strictly inside a pair: snap to `off + min(1,
//!   max(clen, rem))` — i.e. the start of the post-image segment
//!   when either side is non-empty, or `off` when both are empty.
//! - `position` at the trailing edge of a pair: snap to `off +
//!   clen` (the end of the post-image segment).
//!
//! ## Divergence
//!
//! The Lisp consumes the raw output of `match-diff` /
//! `match-readings` and dispatches by `(if (atom part) ...)`. The
//! Rust port consumes [`super::kani_match_part::KaniMatchPart`], an
//! explicit two-variant enum carrying pre-computed character
//! lengths — the function only ever calls `length` on the
//! substrings, so the substrings themselves are dropped at
//! conversion time. Observable behavior is identical.

use super::kani_match_part::KaniMatchPart;

pub fn translate_hint_position(matched: &[KaniMatchPart], position: usize) -> Option<usize> {
    let mut off: usize = 0;
    let mut rem: usize = position;
    for part in matched {
        match part {
            KaniMatchPart::Atom(len) => {
                if rem <= *len {
                    return Some(off + rem);
                }
                rem -= *len;
                off += *len;
            }
            KaniMatchPart::Pair(len, clen) => {
                if rem < *len {
                    return Some(off + 1usize.min(rem.max(*clen)));
                }
                if rem == *len {
                    return Some(off + *clen);
                }
                rem -= *len;
                off += *clen;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Empty alignment: any position other than 0 overshoots and
    /// returns `None`. Position 0 also returns `None` because the
    /// `loop` never enters its body and the upstream returns `nil`.
    #[test]
    fn empty_alignment_returns_none() {
        assert_eq!(translate_hint_position(&[], 0), None);
        assert_eq!(translate_hint_position(&[], 5), None);
    }

    /// Single Atom: positions 0..=len map identity.
    #[test]
    fn atom_identity_map() {
        let m = [KaniMatchPart::Atom(3)];
        assert_eq!(translate_hint_position(&m, 0), Some(0));
        assert_eq!(translate_hint_position(&m, 1), Some(1));
        assert_eq!(translate_hint_position(&m, 3), Some(3));
        assert_eq!(translate_hint_position(&m, 4), None);
    }

    /// Pair snap: position strictly inside a pair returns `off + 1`
    /// when `clen >= 1`. (Upstream: (min 1 (max clen rem)) with
    /// rem >= 1 and clen >= 1 → 1.)
    #[test]
    fn pair_inside_snaps_to_one_when_post_nonempty() {
        let m = [KaniMatchPart::Pair(3, 2)];
        assert_eq!(translate_hint_position(&m, 1), Some(1));
        assert_eq!(translate_hint_position(&m, 2), Some(1));
    }

    /// Pair snap at trailing edge: position == len returns
    /// `off + clen` (end of post-image segment).
    #[test]
    fn pair_trailing_edge_returns_clen() {
        let m = [KaniMatchPart::Pair(3, 2)];
        assert_eq!(translate_hint_position(&m, 3), Some(2));
    }

    /// Both sides empty (`Pair(0, 0)`): inside-branch yields
    /// `off + 0`; the loop then advances by (0, 0) and continues.
    /// This is the same `< rem len` branch that fires when rem=0.
    #[test]
    fn pair_zero_zero_yields_off() {
        let m = [KaniMatchPart::Atom(2), KaniMatchPart::Pair(0, 0)];
        // rem=2 falls through atom, then rem=0 < len=0 is false,
        // rem=0 == len=0 hits the second branch, returns off+clen=2+0=2.
        assert_eq!(translate_hint_position(&m, 2), Some(2));
    }

    /// rem=0, Pair(0, 1): first branch `rem < len` is false (0 < 0
    /// is false). Second branch `rem == len` is true, returns
    /// off + clen = 0 + 1 = 1.
    #[test]
    fn pair_zero_post_one_at_position_zero() {
        let m = [KaniMatchPart::Pair(0, 1)];
        assert_eq!(translate_hint_position(&m, 0), Some(1));
    }

    /// rem=0, Pair(1, 0): first branch `rem < len` is true (0 < 1).
    /// max(clen=0, rem=0) = 0, min(1, 0) = 0. Returns off + 0.
    #[test]
    fn pair_one_post_zero_at_position_zero() {
        let m = [KaniMatchPart::Pair(1, 0)];
        assert_eq!(translate_hint_position(&m, 0), Some(0));
    }

    /// Multi-segment walk: Atom(2) + Pair(2, 3) + Atom(1).
    /// Total pre-length = 5; post-length = 6.
    #[test]
    fn multi_segment_walk() {
        let m = [
            KaniMatchPart::Atom(2),
            KaniMatchPart::Pair(2, 3),
            KaniMatchPart::Atom(1),
        ];
        // 0..=2 in the atom: identity
        assert_eq!(translate_hint_position(&m, 0), Some(0));
        assert_eq!(translate_hint_position(&m, 2), Some(2));
        // Pair pre-region 2..4: snap to atom-end + 1 = 3
        assert_eq!(translate_hint_position(&m, 3), Some(3));
        // Pair trailing edge: rem=2, len=2 → off + clen = 2 + 3 = 5
        assert_eq!(translate_hint_position(&m, 4), Some(5));
        // Trailing atom: at position=5, the pair "otherwise" branch
        // advances rem to 1 and off to 5; the Atom(1) returns
        // off+rem = 6 — the end of the entire post-image.
        assert_eq!(translate_hint_position(&m, 5), Some(6));
        // position=6 overshoots
        assert_eq!(translate_hint_position(&m, 6), None);
    }
}
