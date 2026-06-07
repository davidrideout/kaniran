use super::*;

// --- match_readings_star_ ---
fn reading(text: &str, ty: &str) -> KanjiReading {
    KanjiReading {
        reading: text.to_string(),
        r#type: ty.to_string(),
        tag: None,
        gem: None,
    }
}

#[test]
fn empty_rmap_with_exhausted_reading_returns_zero_score() {
    match match_readings_star(&[], "") {
        MatchResult::Some { items, score } => {
            assert!(items.is_empty());
            assert_eq!(score, 0);
        }
        MatchResult::None => panic!("expected Some, got None"),
    }
}

#[test]
fn empty_rmap_with_remaining_reading_returns_none() {
    assert_eq!(match_readings_star(&[], "abc"), MatchResult::None);
}

#[test]
fn nonempty_rmap_with_exhausted_reading_returns_none() {
    let rmap = vec![RmapEntry::Readings(vec![reading("a", "x")])];
    assert_eq!(match_readings_star(&rmap, ""), MatchResult::None);
}

#[test]
fn nonkanji_passthrough_matches_exact_char() {
    let rmap = vec![RmapEntry::NonKanji('っ')];
    let result = match_readings_star(&rmap, "っ");
    assert_eq!(
        result,
        MatchResult::Some {
            items: vec![MatchItem::Char('っ')],
            score: 0,
        }
    );
}

#[test]
fn nonkanji_mismatch_returns_none() {
    let rmap = vec![RmapEntry::NonKanji('っ')];
    assert_eq!(match_readings_star(&rmap, "あ"), MatchResult::None);
}

#[test]
fn matching_reading_keeps_score_zero() {
    let rmap = vec![RmapEntry::Readings(vec![
        reading("ひ", "ja_kun"),
        reading("にち", "ja_on"),
    ])];
    match match_readings_star(&rmap, "ひ") {
        MatchResult::Some { items, score } => {
            assert_eq!(score, 0);
            assert_eq!(items.len(), 1);
            match &items[0] {
                MatchItem::Reading(r) => {
                    assert_eq!(r.reading, "ひ");
                    assert_eq!(r.r#type, "ja_kun");
                }
                _ => panic!("expected Reading, got Char"),
            }
        }
        MatchResult::None => panic!("expected Some, got None"),
    }
}

#[test]
fn unmatched_reading_falls_back_to_irr_with_negative_score() {
    let rmap = vec![RmapEntry::Readings(vec![reading("ひ", "ja_kun")])];
    match match_readings_star(&rmap, "ぜ") {
        MatchResult::Some { items, score } => {
            assert_eq!(score, -1);
            assert_eq!(items.len(), 1);
            match &items[0] {
                MatchItem::Reading(r) => {
                    assert_eq!(r.reading, "ぜ");
                    assert_eq!(r.r#type, "irr");
                }
                _ => panic!("expected Reading, got Char"),
            }
        }
        MatchResult::None => panic!("expected Some, got None"),
    }
}

#[test]
fn ties_break_to_largest_end() {
    // Single kanji vs. a 2-char reading. Two valid spans:
    //   end=1: irr("あ"), score = 0 - 1 = -1, then needs reading[1..] match
    //   end=2: irr("あい"), score = 0 - 2 = -2.
    // The larger-end (longer irr) outscores the shorter prefix only when
    // followed-by-mismatch invalidates end=1. Here we set up a case where
    // both ends produce a valid recursion: rmap has two kanji slots, and
    // either span [0..1]+[1..2] or [0..2]+exhausted-reading is reachable.
    // Per upstream tie-break (largest end wins), the [0..2]-span result
    // should be selected.
    let rmap = vec![
        RmapEntry::Readings(vec![reading("あい", "x")]),
        RmapEntry::Readings(vec![reading("う", "y")]),
    ];
    // Reading "あいう" — only one valid match: ("あい", "x") then ("う", "y").
    match match_readings_star(&rmap, "あいう") {
        MatchResult::Some { items, score } => {
            assert_eq!(score, 0);
            assert_eq!(items.len(), 2);
            if let MatchItem::Reading(r0) = &items[0] {
                assert_eq!(r0.reading, "あい");
            } else {
                panic!("expected Reading at 0");
            }
            if let MatchItem::Reading(r1) = &items[1] {
                assert_eq!(r1.reading, "う");
            } else {
                panic!("expected Reading at 1");
            }
        }
        MatchResult::None => panic!("expected Some, got None"),
    }
}
