use super::super::helpers::dakuten_join;
use super::*;
use SegmentKind::*;

// --- simplify_ngrams ---
/// `"か゛"` (ka + combining dakuten) folds to `"が"`.
#[test]
fn folds_combining_dakuten_via_runtime_map() {
    assert_eq!(simplify_ngrams("か゛", dakuten_join()), "が");
    assert_eq!(simplify_ngrams("ハ゜", dakuten_join()), "パ");
}

/// Empty map is a no-op.
#[test]
fn empty_map_returns_input_unchanged() {
    let map: &[(&str, &str)] = &[];
    assert_eq!(simplify_ngrams("hello", map), "hello");
}

// --- basic_split ---
#[test]
fn alternates_misc_and_word_segments() {
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
fn pure_japanese_is_one_word_segment() {
    assert_eq!(basic_split("日本語"), vec![(Word, "日本語".to_string())]);
}

// --- consecutive_char_groups ---
/// Positions are character offsets, not byte offsets.
#[test]
fn returns_character_offsets_not_byte_offsets() {
    let s = "あ12い34";
    // chars: 0='あ' 1='1' 2='2' 3='い' 4='3' 5='4'
    assert_eq!(
        consecutive_char_groups(CharClass::Number, s, 0, s.chars().count()),
        vec![(1, 3), (4, 6)],
    );
}
