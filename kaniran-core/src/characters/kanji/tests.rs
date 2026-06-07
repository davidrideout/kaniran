use super::*;

// --- sequential_kanji_positions ---
/// Lookahead semantics: a run of N kanji yields N-1 positions, each
/// pointing to the *second* kanji of an adjacent pair (char index).
#[test]
fn returns_char_position_of_second_in_each_adjacent_pair() {
    assert_eq!(sequential_kanji_positions("日本語", 0), vec![1, 2]);
    assert_eq!(sequential_kanji_positions("日本語", 5), vec![6, 7]);
}

/// Non-kanji separators break adjacency.
#[test]
fn non_kanji_breaks_adjacency() {
    assert_eq!(sequential_kanji_positions("日の本", 0), Vec::<usize>::new());
    assert_eq!(
        sequential_kanji_positions("ひらがな", 0),
        Vec::<usize>::new()
    );
}

// --- kanji_regex ---
/// Pure-kanji word collapses to `^.+$`: any non-empty reading accepted, empty rejected.
#[test]
fn pure_kanji_word_accepts_any_nonempty_reading() {
    let re = kanji_regex("日本語");
    assert!(re.is_match("にほんご").unwrap());
    assert!(!re.is_match("").unwrap());
}

/// Non-kanji characters in the word stay literal in the regex.
#[test]
fn non_kanji_characters_stay_literal() {
    let re = kanji_regex("お茶");
    assert!(re.is_match("おちゃ").unwrap());
    assert!(!re.is_match("にちゃ").unwrap());
}

// --- kanji_prefix ---
/// No kanji → empty string.
#[test]
fn returns_empty_when_no_kanji() {
    assert_eq!(kanji_prefix("ひらがな"), "");
    assert_eq!(kanji_prefix(""), "");
}

/// Returns up to and including the last kanji; trailing non-kanji dropped.
#[test]
fn returns_prefix_up_to_last_kanji() {
    assert_eq!(kanji_prefix("お茶を飲む"), "お茶を飲");
    assert_eq!(kanji_prefix("日本語"), "日本語");
}
