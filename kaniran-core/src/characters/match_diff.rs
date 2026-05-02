//! Port of `ichiran/characters:match-diff` (`characters.lisp:326-357`).
//!
//! Recursively align two strings into a sequence of [`MatchSegment`]s
//! that alternate between equal regions and pairs of differing regions,
//! returning also a score equal to the total length of the matched
//! portions (number of *characters*, not bytes). Used by the kanji
//! reading-matcher for any non-kanji string pair.
//!
//! Returns `None` when either input is empty. The upstream `cond` for
//! these branches is `((zerop l1))` / `((zerop l2))` with no body —
//! which means the cond returns `T` (the value of `(zerop ...)` when
//! true), not `nil`. The Rust port deliberately returns `None` instead
//! of trying to surface that `T` sentinel, because (a) `T` carries no
//! useful data, and (b) the only non-recursive caller
//! (`dict-split.lisp:933`) only ever passes non-empty strings, so no
//! upstream code reads the `T` return.
//!
//! Algorithmic notes:
//! - When one input has length 1 (and the inputs differ), the result
//!   is a single `Diff(s1, s2)` segment with score 0 — even when the
//!   first characters happen to match. The upstream early-returns this
//!   case before the prefix-match branches fire.
//! - With no shared prefix, the function does a naive O(l1·l2)
//!   pair-search, recursing on every matching `(i, j)` pair to pick the
//!   alignment with the highest score. Worst-case complexity is
//!   exponential; the upstream relies on the inputs being short kana
//!   strings.

use super::safe_subseq::safe_subseq;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchSegment {
    Equal(String),
    Diff(String, String),
}

pub fn match_diff(s1: &str, s2: &str) -> Option<(Vec<MatchSegment>, usize)> {
    let c1: Vec<char> = s1.chars().collect();
    let c2: Vec<char> = s2.chars().collect();
    let l1 = c1.len();
    let l2 = c2.len();
    if l1 == 0 || l2 == 0 {
        return None;
    }
    let m = mismatch(&c1, &c2);
    match m {
        None => Some((vec![MatchSegment::Equal(s1.to_string())], l1)),
        Some(_) if l1 == 1 || l2 == 1 => Some((
            vec![MatchSegment::Diff(s1.to_string(), s2.to_string())],
            0,
        )),
        Some(0) => {
            let mut best: Option<(Vec<MatchSegment>, usize)> = None;
            for i in 1..l1 {
                for j in 1..l2 {
                    if c1[i] != c2[j] {
                        continue;
                    }
                    let s1_rest = safe_subseq(s1, i, None).expect("i < l1");
                    let s2_rest = safe_subseq(s2, j, None).expect("j < l2");
                    let Some((rest_match, rest_value)) = match_diff(&s1_rest, &s2_rest) else {
                        continue;
                    };
                    let is_better = best.as_ref().map_or(true, |(_, v)| rest_value > *v);
                    if is_better {
                        let head = MatchSegment::Diff(
                            safe_subseq(s1, 0, Some(i)).expect("i <= l1"),
                            safe_subseq(s2, 0, Some(j)).expect("j <= l2"),
                        );
                        let mut combined = vec![head];
                        combined.extend(rest_match);
                        best = Some((combined, rest_value));
                    }
                }
            }
            best
        }
        Some(m) if m == l1 => {
            // s1 is a strict prefix of s2. The upstream peels the last
            // matched character into the Diff, so the Equal segment is
            // l1-1 long and the Diff carries one s1 character versus
            // the trailing s2 portion.
            Some((
                vec![
                    MatchSegment::Equal(safe_subseq(s1, 0, Some(l1 - 1)).expect("l1 >= 1")),
                    MatchSegment::Diff(
                        safe_subseq(s1, l1 - 1, None).expect("l1 >= 1"),
                        safe_subseq(s2, l1 - 1, None).expect("l1-1 < l2"),
                    ),
                ],
                l1 - 1,
            ))
        }
        Some(m) if m == l2 => Some((
            vec![
                MatchSegment::Equal(safe_subseq(s2, 0, Some(l2 - 1)).expect("l2 >= 1")),
                MatchSegment::Diff(
                    safe_subseq(s1, l2 - 1, None).expect("l2-1 < l1"),
                    safe_subseq(s2, l2 - 1, None).expect("l2 >= 1"),
                ),
            ],
            l2 - 1,
        )),
        Some(m) => {
            let s1_rest = safe_subseq(s1, m, None).expect("m < l1");
            let s2_rest = safe_subseq(s2, m, None).expect("m < l2");
            match_diff(&s1_rest, &s2_rest).map(|(rest_match, rest_value)| {
                let head = MatchSegment::Equal(safe_subseq(s1, 0, Some(m)).expect("m <= l1"));
                let mut combined = vec![head];
                combined.extend(rest_match);
                (combined, rest_value + m)
            })
        }
    }
}

fn mismatch(a: &[char], b: &[char]) -> Option<usize> {
    let shared = a.len().min(b.len());
    for i in 0..shared {
        if a[i] != b[i] {
            return Some(i);
        }
    }
    if a.len() == b.len() {
        None
    } else {
        Some(shared)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_none() {
        assert_eq!(match_diff("", "abc"), None);
        assert_eq!(match_diff("abc", ""), None);
        assert_eq!(match_diff("", ""), None);
    }

    #[test]
    fn equal_strings_collapse_to_one_equal_segment() {
        assert_eq!(
            match_diff("abc", "abc"),
            Some((vec![MatchSegment::Equal("abc".into())], 3))
        );
    }

    #[test]
    fn single_char_difference_is_one_diff_segment() {
        assert_eq!(
            match_diff("a", "b"),
            Some((vec![MatchSegment::Diff("a".into(), "b".into())], 0))
        );
    }

    /// Common prefix + differing suffix: produces alternating Equal /
    /// Diff. Score is the length of the matched prefix (1 char).
    #[test]
    fn shared_prefix_then_diff() {
        assert_eq!(
            match_diff("ab", "ac"),
            Some((
                vec![
                    MatchSegment::Equal("a".into()),
                    MatchSegment::Diff("b".into(), "c".into()),
                ],
                1,
            ))
        );
    }

    /// CJK char-position semantics — the score and segment offsets are
    /// in characters, not bytes. Without this, an accidental byte-index
    /// implementation passes every ASCII test.
    #[test]
    fn cjk_alignment_uses_char_positions() {
        // 3 chars on each side; first matches, second differs.
        // 日 matches (+1), 本/中 differs (+0), 語 matches (+1) — total 2.
        let result = match_diff("日本語", "日中語").expect("non-empty");
        assert_eq!(
            result,
            (
                vec![
                    MatchSegment::Equal("日".into()),
                    MatchSegment::Diff("本".into(), "中".into()),
                    MatchSegment::Equal("語".into()),
                ],
                2,
            )
        );
    }
}
