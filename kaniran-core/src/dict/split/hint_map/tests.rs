mod _star_hint_map_star_ {
    use crate::dict::split::hint_map::*;

    /// EASY_HINTS has exactly 431 entries, matching the
    /// `def-easy-hint` callsite count in `dict-split.lisp`.
    #[test]
    fn easy_hints_count_matches_upstream() {
        assert_eq!(EASY_HINTS.len(), 431);
    }

    /// All easy-hint kanji-splits are non-empty (smoke-test against
    /// a regex/parser miss in the data extraction).
    #[test]
    fn easy_hints_no_empty_strings() {
        for e in EASY_HINTS {
            assert!(
                !e.kanji_split.is_empty(),
                "empty kanji_split for seq {}",
                e.seq
            );
        }
    }

    // (`EASY_HINTS_SEQS` ↔ `EASY_HINTS` agreement is now structural —
    // `easy_hints_seqs()` derives directly from `EASY_HINTS` via
    // OnceLock per CONVENTIONS §5.2, so they cannot disagree.)

    #[test]
    fn search_chars_finds_first() {
        assert_eq!(search_chars("は", "こんにちはまた", false), Some(4));
    }

    #[test]
    fn search_chars_from_end_finds_last() {
        assert_eq!(search_chars("は", "はは", true), Some(1));
    }

    #[test]
    fn search_chars_substring_multi_char() {
        assert_eq!(search_chars("では", "それではない", true), Some(2));
    }

    #[test]
    fn search_chars_missing_returns_none() {
        assert_eq!(search_chars("は", "こんに", false), None);
    }

    #[test]
    fn ends_with_char_basic() {
        assert!(ends_with_char("こんにちは", 'は'));
        assert!(!ends_with_char("こんにちは!", 'は'));
        assert!(!ends_with_char("", 'は'));
    }

    #[test]
    fn safe_hint_drops_negative() {
        assert_eq!(safe_hint(KaniHintKind::Mod, -1), None);
        assert_eq!(
            safe_hint(KaniHintKind::Mod, 0),
            Some((KaniHintKind::Mod, 0))
        );
        assert_eq!(
            safe_hint(KaniHintKind::Space, 5),
            Some((KaniHintKind::Space, 5))
        );
    }
}
