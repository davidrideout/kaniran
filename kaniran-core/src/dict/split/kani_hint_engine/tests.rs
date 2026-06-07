mod kani_hint_engine {
    use crate::dict::split::kani_hint_engine::*;

    /// "郷 に 入って は 郷 に 従え" — parts = ["郷", "に", "入って",
    /// "は", "郷", "に", "従え"]. Joined text = "郷に入っては郷に従え"
    /// (10 chars). Hints fire at every interior space (pos=1,2,5,6,7,8)
    /// plus the `は`-mod at pos `5 + 1 - 1 = 5` (は starts at pos 5
    /// with part_len = 1).
    #[test]
    fn parse_typical_easy_hint() {
        let (text, hints) = parse_kanji_split("郷 に 入って は 郷 に 従え");
        assert_eq!(text, "郷に入っては郷に従え");
        assert_eq!(
            hints,
            vec![
                (KaniHintKind::Space, 1), // before に
                (KaniHintKind::Space, 2), // before 入って
                (KaniHintKind::Space, 5), // before は
                (KaniHintKind::Mod, 5),   // は's mod
                (KaniHintKind::Space, 6), // before 郷
                (KaniHintKind::Space, 7), // before に
                (KaniHintKind::Space, 8), // before 従え
            ]
        );
    }

    /// "とは" appears in trigger set — emits :mod at pos + len - 1
    /// when starting at offset 0.
    #[test]
    fn parse_with_toha_emits_mod() {
        let (text, hints) = parse_kanji_split("とは 言うものの");
        assert_eq!(text, "とは言うものの");
        assert_eq!(hints, vec![(KaniHintKind::Space, 2),]);
        // ↑ no :mod for "とは" at index 0 because the macro's
        // `unless (zerop pos)` gates BOTH the space and the mod emit
        // (the `and if` clause runs only when the unless succeeds).
    }

    /// Single-part: no interior space, no hints.
    #[test]
    fn parse_single_part_emits_no_hints() {
        let (text, hints) = parse_kanji_split("おはよう");
        assert_eq!(text, "おはよう");
        assert!(hints.is_empty());
    }

    /// "は" inside emits a :mod at pos + len - 1 = pos (len("は")=1).
    /// Verified against the upstream macroexpansion of
    /// `(def-easy-hint 1338260 "出る 釘 は 打たれる")` (REPL).
    #[test]
    fn parse_ha_in_middle() {
        let (text, hints) = parse_kanji_split("出る 釘 は 打たれる");
        assert_eq!(text, "出る釘は打たれる");
        // parts: 出る(2) 釘(1) は(1) 打たれる(4); pos values at each
        // interior boundary: 2 (before 釘), 3 (before は), 4 (before
        // 打たれる). The は part also emits :mod at pos=3.
        assert_eq!(
            hints,
            vec![
                (KaniHintKind::Space, 2),
                (KaniHintKind::Space, 3),
                (KaniHintKind::Mod, 3),
                (KaniHintKind::Space, 4),
            ]
        );
    }
}
