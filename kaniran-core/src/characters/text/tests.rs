use super::*;

// --- match_diff ---
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

/// Common prefix + differing suffix produces alternating Equal / Diff.
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

/// Score and segment offsets are in characters, not bytes.
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

// --- safe_subseq ---
/// Slicing covers char-indexed CJK input correctly.
#[test]
fn slices_by_character_not_byte() {
    let s = "あいうえお";
    assert_eq!(safe_subseq(s, 1, Some(4)).as_deref(), Some("いうえ"));
    assert_eq!(safe_subseq(s, 0, None).as_deref(), Some("あいうえお"));
}

/// Out-of-range `start`, `end`, or `start > end` all return None.
#[test]
fn rejects_out_of_range_or_inverted() {
    let s = "abc";
    assert_eq!(safe_subseq(s, 4, None), None);
    assert_eq!(safe_subseq(s, 0, Some(4)), None);
    assert_eq!(safe_subseq(s, 2, Some(1)), None);
}
