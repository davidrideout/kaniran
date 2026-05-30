//! String-level utilities: regex splitting, basic tokenization, kanji
//! run discovery, mora counting, stem trimming, the recursive aligner,
//! safe substring, and the join helper. From `characters.lisp:234-249`
//! (split / basic-split / mora-length), `:273-278`
//! (consecutive-char-groups), and `:316-370` (destem / match-diff /
//! safe-subseq / join).

use std::sync::OnceLock;

use fancy_regex::Regex;

use super::char_classes::{
    basic_split_regex, char_class_bare_scanners, char_scanners_inner, CharClass,
};

/// `split-by-regex` (`characters.lisp:234-236`). Split `s` by `regex`,
/// interleaving the captured groups with the between-match text and
/// dropping any empty pieces. Mirrors cl-ppcre's
/// `(ppcre:split regex str :with-registers-p t)`.
///
/// With a regex that has a single outer capture group (the upstream
/// usage in `*basic-split-regex*`), the result alternates
/// "between-match text" with "the matched text" — exactly what
/// [`basic_split`] relies on for its misc/word classification.
pub fn split_by_regex(regex: &Regex, s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut last = 0;
    for cap in regex.captures_iter(s) {
        let cap = cap.expect("regex iteration error");
        let m = cap.get(0).expect("capture 0 is the whole match");
        let before = &s[last..m.start()];
        if !before.is_empty() {
            out.push(before.to_string());
        }
        for i in 1..cap.len() {
            if let Some(g) = cap.get(i) {
                let g = g.as_str();
                if !g.is_empty() {
                    out.push(g.to_string());
                }
            }
        }
        last = m.end();
    }
    let after = &s[last..];
    if !after.is_empty() {
        out.push(after.to_string());
    }
    out
}

/// Tag for the segments returned by [`basic_split`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    Misc,
    Word,
}

/// `basic-split` (`characters.lisp:238-243`). Segment Japanese-mixed
/// text into alternating misc / word runs. Splits with
/// `*basic-split-regex*` (compiled lazily here), tags the first segment
/// via `test_word(.., :nonword)`, and alternates from there — that's
/// how upstream encodes "consecutive segments returned by
/// `:with-registers-p t` alternate between between-match misc text and
/// matched word groups."
pub fn basic_split(s: &str) -> Vec<(SegmentKind, String)> {
    static SCANNER: OnceLock<Regex> = OnceLock::new();
    let scanner = SCANNER
        .get_or_init(|| Regex::new(basic_split_regex()).expect("basic-split-regex must compile"));
    let segments = split_by_regex(scanner, s);
    let mut prev: Option<bool> = None;
    segments
        .into_iter()
        .map(|seg| {
            let misc = match prev {
                None => super::char_classes::test_word(&seg, CharClass::Nonword),
                Some(p) => !p,
            };
            prev = Some(misc);
            (
                if misc { SegmentKind::Misc } else { SegmentKind::Word },
                seg,
            )
        })
        .collect()
}

/// `mora-length` (`characters.lisp:245-249`). Counts the number of
/// "real" morae in a kana string, ignoring the sokuon, all small kana
/// modifiers (`ぁィゥェォ`, `ャュョ`), and the long-vowel mark `ー`. Each
/// excluded glyph either fuses with or lengthens its neighbour rather
/// than contributing a mora of its own.
pub fn mora_length(s: &str) -> usize {
    const MODIFIERS: &str = "っッぁァぃィぅゥぇェぉォゃャゅュょョー";
    s.chars().filter(|c| !MODIFIERS.contains(*c)).count()
}

/// `consecutive-char-groups` (`characters.lisp:273-278`). Find every run
/// of consecutive characters in `char_class` within `s[start..end]` and
/// return each as a `(start, end)` pair.
///
/// Positions are *character* offsets, matching the upstream — not byte
/// offsets — so callers can pass them through `chars().nth()` or
/// compare against Lisp fixtures without translation. The function
/// converts to byte offsets internally to drive the regex, then
/// converts back.
pub fn consecutive_char_groups(
    char_class: CharClass,
    s: &str,
    start: usize,
    end: usize,
) -> Vec<(usize, usize)> {
    let re = char_scanners_inner()
        .get(&char_class)
        .expect("char_class is in *char-scanners-inner*");
    let start_byte = nth_char_byte(s, start);
    let end_byte = nth_char_byte(s, end);
    let slice = &s[start_byte..end_byte];
    re.find_iter(slice)
        .map(|m| m.expect("regex iteration"))
        .map(|m| {
            let s_char = start + slice[..m.start()].chars().count();
            let e_char = start + slice[..m.end()].chars().count();
            (s_char, e_char)
        })
        .collect()
}

fn nth_char_byte(s: &str, char_pos: usize) -> usize {
    s.char_indices()
        .nth(char_pos)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

/// `destem` (`characters.lisp:316-324`). Trim from the end of `word` the
/// suffix that begins at the `stem`-th match of `char_class`'s pattern
/// (counted from the end). `stem == 0` returns `word` unchanged. When
/// `word` has fewer than `stem` matches of the class, the result is
/// empty.
///
/// Char-position semantics (CONVENTIONS §4.5): the cut is on a
/// character boundary, so multi-byte code points pass through
/// correctly. The compiled regex comes from
/// [`super::char_classes::char_class_bare_scanners`].
///
/// Diverges from the Lisp by exposing `char_class` as a required
/// argument (the Lisp `&optional` defaulted to `:kana`); both upstream
/// call-sites pass the default explicitly under this convention.
pub fn destem(word: &str, stem: usize, char_class: CharClass) -> String {
    if stem == 0 {
        return word.to_string();
    }
    let re = char_class_bare_scanners()
        .get(&char_class)
        .expect("char_class is in *char-class-regex-mapping*");
    let positions: Vec<usize> = re
        .find_iter(word)
        .map(|m| m.expect("regex iteration"))
        .map(|m| word[..m.start()].chars().count())
        .collect();
    if stem > positions.len() {
        return String::new();
    }
    let cut = positions[positions.len() - stem];
    word.chars().take(cut).collect()
}

/// Alternating segment returned by [`match_diff`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchSegment {
    Equal(String),
    Diff(String, String),
}

/// `match-diff` (`characters.lisp:326-357`). Recursively align two
/// strings into a sequence of [`MatchSegment`]s that alternate between
/// equal regions and pairs of differing regions, returning also a score
/// equal to the total length of the matched portions (number of
/// *characters*, not bytes). Used by the kanji reading-matcher for any
/// non-kanji string pair.
///
/// Returns `None` when either input is empty. The upstream `cond` for
/// these branches is `((zerop l1))` / `((zerop l2))` with no body —
/// which means the cond returns `T` (the value of `(zerop ...)` when
/// true), not `nil`. The Rust port deliberately returns `None` instead
/// of trying to surface that `T` sentinel, because (a) `T` carries no
/// useful data, and (b) the only non-recursive caller
/// (`dict-split.lisp:933`) only ever passes non-empty strings, so no
/// upstream code reads the `T` return.
///
/// Algorithmic notes:
/// - When one input has length 1 (and the inputs differ), the result
///   is a single `Diff(s1, s2)` segment with score 0 — even when the
///   first characters happen to match. The upstream early-returns this
///   case before the prefix-match branches fire.
/// - With no shared prefix, the function does a naive O(l1·l2)
///   pair-search, recursing on every matching `(i, j)` pair to pick the
///   alignment with the highest score. Worst-case complexity is
///   exponential; the upstream relies on the inputs being short kana
///   strings.
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

/// `safe-subseq` (`characters.lisp:359-363`). Bounds-checked substring.
/// Returns `None` when `start` or `end` would fall outside
/// `[0, char-length(s)]`, or when `start > end`. Otherwise returns
/// `Some(<chars start..end>)`. With `end = None`, slices to the end of
/// the string.
///
/// Positions are *character* offsets (CONVENTIONS §4.5) — matches the
/// Lisp's `subseq` semantics on SBCL strings.
pub fn safe_subseq(s: &str, start: usize, end: Option<usize>) -> Option<String> {
    let len = s.chars().count();
    if start > len {
        return None;
    }
    let stop = match end {
        Some(e) if e > len || start > e => return None,
        Some(e) => e,
        None => len,
    };
    Some(s.chars().skip(start).take(stop - start).collect())
}

/// `join` (`characters.lisp:365-370`). Concatenate `items` with
/// `separator` between each pair. The Lisp `&key key` parameter is
/// dropped — its single upstream caller (`numbers.lisp:136`) pre-maps
/// already, and Rust callers that need per-element transformation can
/// `.iter().map(...).collect::<Vec<_>>()` before passing in. The
/// generic bound lets callers pass `&[&str]`, `&[String]`, etc.
/// interchangeably.
pub fn join<S: AsRef<str>>(separator: &str, items: &[S]) -> String {
    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push_str(separator);
        }
        out.push_str(item.as_ref());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use SegmentKind::*;

    #[test]
    fn basic_split_alternates_misc_and_word_segments() {
        assert_eq!(
            basic_split("hello 日本 world"),
            vec![
                (Misc, "hello ".to_string()),
                (Word, "日本".to_string()),
                (Misc, " world".to_string()),
            ]
        );
    }

    #[test]
    fn basic_split_pure_japanese_is_one_word_segment() {
        assert_eq!(basic_split("日本語"), vec![(Word, "日本語".to_string())]);
    }

    /// Positions are character offsets, not byte offsets. With
    /// 3-byte CJK chars this is the only behavior worth pinning;
    /// everything else is delegated to the regex.
    #[test]
    fn consecutive_char_groups_returns_character_offsets_not_byte_offsets() {
        let s = "あ12い34";
        // chars: 0='あ' 1='1' 2='2' 3='い' 4='3' 5='4'
        assert_eq!(
            consecutive_char_groups(CharClass::Number, s, 0, s.chars().count()),
            vec![(1, 3), (4, 6)],
        );
    }

    #[test]
    fn match_diff_empty_input_returns_none() {
        assert_eq!(match_diff("", "abc"), None);
        assert_eq!(match_diff("abc", ""), None);
        assert_eq!(match_diff("", ""), None);
    }

    #[test]
    fn match_diff_equal_strings_collapse_to_one_equal_segment() {
        assert_eq!(
            match_diff("abc", "abc"),
            Some((vec![MatchSegment::Equal("abc".into())], 3))
        );
    }

    #[test]
    fn match_diff_single_char_difference_is_one_diff_segment() {
        assert_eq!(
            match_diff("a", "b"),
            Some((vec![MatchSegment::Diff("a".into(), "b".into())], 0))
        );
    }

    /// Common prefix + differing suffix: produces alternating Equal /
    /// Diff. Score is the length of the matched prefix (1 char).
    #[test]
    fn match_diff_shared_prefix_then_diff() {
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
    fn match_diff_cjk_alignment_uses_char_positions() {
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

    /// Slicing covers char-indexed CJK input correctly — without
    /// pinning this, an accidental byte-index implementation passes
    /// every ASCII test.
    #[test]
    fn safe_subseq_slices_by_character_not_byte() {
        let s = "あいうえお";
        assert_eq!(safe_subseq(s, 1, Some(4)).as_deref(), Some("いうえ"));
        assert_eq!(safe_subseq(s, 0, None).as_deref(), Some("あいうえお"));
    }

    /// Out-of-range `start`, `end`, or `start > end` all return None,
    /// mirroring the Lisp's `(when ...)` guard.
    #[test]
    fn safe_subseq_rejects_out_of_range_or_inverted() {
        let s = "abc";
        assert_eq!(safe_subseq(s, 4, None), None);
        assert_eq!(safe_subseq(s, 0, Some(4)), None);
        assert_eq!(safe_subseq(s, 2, Some(1)), None);
    }
}
