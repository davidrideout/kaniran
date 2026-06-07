mod process_hints {
    use crate::dict::split::hint::*;
    use crate::dict::split::segsplit::KANA_HINT_MOD;
    use crate::dict::split::segsplit::KANA_HINT_SPACE;

    /// `*kana-hint-mod*` + `は` → `わ`. The canonical rewrite the
    /// hint system exists to enable.
    #[test]
    fn mod_plus_ha_becomes_wa() {
        let input = format!("こんにち{}は", KANA_HINT_MOD);
        assert_eq!(process_hints(&input), "こんにちわ");
    }

    /// `*kana-hint-mod*` + `へ` → `え`.
    #[test]
    fn mod_plus_he_becomes_e() {
        let input = format!("ところ{}へ", KANA_HINT_MOD);
        assert_eq!(process_hints(&input), "ところえ");
    }

    /// Katakana variants: `*kana-hint-mod*` + `ハ` → `ワ`,
    /// `*kana-hint-mod*` + `ヘ` → `エ`.
    #[test]
    fn katakana_variants() {
        let input = format!("{m}ハ{m}ヘ", m = KANA_HINT_MOD,);
        assert_eq!(process_hints(&input), "ワエ");
    }

    /// `*kana-hint-space*` → ASCII space.
    #[test]
    fn space_sentinel_becomes_ascii_space() {
        let input = format!("ところ{}へ", KANA_HINT_SPACE);
        assert_eq!(process_hints(&input), "ところ へ");
    }

    /// Lone `*kana-hint-mod*` (no following は/ハ/へ/ヘ) drops.
    /// The order in [`hint_simplify_map`] ensures the 2-char rules
    /// fire first so we only fall through to the empty substitution
    /// when no digram matches.
    #[test]
    fn lone_mod_drops() {
        let input = format!("a{}b", KANA_HINT_MOD);
        assert_eq!(process_hints(&input), "ab");
    }

    /// No sentinels — pass-through.
    #[test]
    fn no_sentinels_unchanged() {
        assert_eq!(process_hints("こんにちは"), "こんにちは");
    }
}

mod strip_hints {
    use crate::dict::split::hint::*;
    use crate::dict::split::segsplit::KANA_HINT_MOD;
    use crate::dict::split::segsplit::KANA_HINT_SPACE;

    /// Round-trip with [`crate::dict::insert_hints`]: inserting a
    /// `:mod` sentinel then stripping yields the original kana.
    #[test]
    fn strips_inserted_mod() {
        let with_hint = format!("こんにち{}は", KANA_HINT_MOD);
        assert_eq!(strip_hints(&with_hint), "こんにちは");
    }

    /// Strips both sentinels in one pass.
    #[test]
    fn strips_space_and_mod() {
        let mixed = format!(
            "a{}b{}c{}d",
            KANA_HINT_SPACE, KANA_HINT_MOD, KANA_HINT_SPACE
        );
        assert_eq!(strip_hints(&mixed), "abcd");
    }

    /// Regular ASCII space (U+0020) is not a sentinel and stays.
    #[test]
    fn preserves_regular_space() {
        assert_eq!(strip_hints("a b c"), "a b c");
    }

    /// No sentinels — input passes through unchanged.
    #[test]
    fn no_sentinels_unchanged() {
        assert_eq!(strip_hints("こんにちは"), "こんにちは");
    }

    /// Empty input.
    #[test]
    fn empty_input() {
        assert_eq!(strip_hints(""), "");
    }
}

mod insert_hints {
    use crate::dict::split::hint::*;
    use crate::dict::split::segsplit::KANA_HINT_MOD;
    use crate::dict::split::segsplit::KANA_HINT_SPACE;

    /// Empty hints is a no-op.
    #[test]
    fn empty_hints_returns_input() {
        assert_eq!(insert_hints("こんにちは", &[]), "こんにちは");
    }

    /// `:mod` at position `len-1` inserts the mod sentinel before
    /// the last character — mirrors the simple-hint rule
    /// `(:mod (- l 1))` for the `(2028920 ;; は)` group.
    #[test]
    fn mod_before_last_char() {
        let out = insert_hints("こんにちは", &[(KaniHintKind::Mod, 4)]);
        assert_eq!(out, format!("こんにち{}は", KANA_HINT_MOD));
    }

    /// Position 0 prefixes the sentinel before the entire string.
    #[test]
    fn position_zero_prefixes() {
        let out = insert_hints("は", &[(KaniHintKind::Mod, 0)]);
        assert_eq!(out, format!("{}は", KANA_HINT_MOD));
    }

    /// Position equal to length suffixes after the entire string.
    #[test]
    fn position_equal_to_length_suffixes() {
        let out = insert_hints("ab", &[(KaniHintKind::Space, 2)]);
        assert_eq!(out, format!("ab{}", KANA_HINT_SPACE));
    }

    /// Out-of-range positions are silently dropped.
    #[test]
    fn out_of_range_position_dropped() {
        assert_eq!(insert_hints("ab", &[(KaniHintKind::Mod, 5)]), "ab");
    }

    /// Multiple hints at the same position emit in the supplied
    /// order — verifies the `push` + `reverse` round-trip.
    #[test]
    fn multiple_hints_same_position_keep_supplied_order() {
        let out = insert_hints("ab", &[(KaniHintKind::Space, 1), (KaniHintKind::Mod, 1)]);
        assert_eq!(out, format!("a{}{}b", KANA_HINT_SPACE, KANA_HINT_MOD));
    }

    /// Mixed `:space` + `:mod` at different positions — mirrors a
    /// `def-simple-hint` body with two emits.
    #[test]
    fn mixed_kinds_at_different_positions() {
        let out = insert_hints(
            "ところへ",
            &[(KaniHintKind::Space, 3), (KaniHintKind::Mod, 3)],
        );
        assert_eq!(out, format!("ところ{}{}へ", KANA_HINT_SPACE, KANA_HINT_MOD));
    }
}

mod _star_easy_hints_seqs_star_ {
    use crate::dict::split::hint::*;

    /// Pins the entry count against the upstream observation. If a
    /// future upstream `dict-split.lisp` adds or removes a
    /// `def-easy-hint` form, this test fails until [`EASY_HINTS`]
    /// is regenerated.
    #[test]
    fn entry_count_matches_upstream() {
        assert_eq!(easy_hints_seqs().len(), 431);
    }
}

mod translate_hint_position {
    use crate::dict::split::hint::*;

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

mod translate_hints {
    use crate::dict::split::hint::*;

    /// Empty hints → empty result.
    #[test]
    fn empty_hints_empty_result() {
        let m = [KaniMatchPart::Atom(3)];
        assert!(translate_hints(&m, &[]).is_empty());
    }

    /// Empty alignment + non-empty hints → empty result (every
    /// hint overshoots).
    #[test]
    fn empty_alignment_drops_all() {
        let hints = [(KaniHintKind::Mod, 1), (KaniHintKind::Space, 2)];
        assert!(translate_hints(&[], &hints).is_empty());
    }

    /// Identity walk over a pure Atom: every position passes
    /// through with the same index, hint kind preserved.
    #[test]
    fn atom_identity_passthrough() {
        let m = [KaniMatchPart::Atom(5)];
        let hints = [(KaniHintKind::Mod, 1), (KaniHintKind::Space, 3)];
        assert_eq!(
            translate_hints(&m, &hints),
            vec![(KaniHintKind::Mod, 1), (KaniHintKind::Space, 3)]
        );
    }

    /// Overshoot drops, in-range survives — output order matches
    /// the surviving subset of input order.
    #[test]
    fn overshoot_filters_out() {
        let m = [KaniMatchPart::Atom(2)];
        let hints = [
            (KaniHintKind::Mod, 1),
            (KaniHintKind::Space, 5),
            (KaniHintKind::Mod, 2),
        ];
        assert_eq!(
            translate_hints(&m, &hints),
            vec![(KaniHintKind::Mod, 1), (KaniHintKind::Mod, 2)]
        );
    }

    /// Walks a mixed Atom/Pair alignment — the position semantics
    /// follow [`translate_hint_position`] (verified there).
    #[test]
    fn mixed_alignment_projection() {
        let m = [
            KaniMatchPart::Atom(2),
            KaniMatchPart::Pair(2, 3),
            KaniMatchPart::Atom(1),
        ];
        let hints = [
            (KaniHintKind::Mod, 1),   // atom-interior: 1
            (KaniHintKind::Space, 3), // pair-interior: 3
            (KaniHintKind::Mod, 4),   // pair-trailing: 5
        ];
        assert_eq!(
            translate_hints(&m, &hints),
            vec![
                (KaniHintKind::Mod, 1),
                (KaniHintKind::Space, 3),
                (KaniHintKind::Mod, 5),
            ]
        );
    }
}

mod _star_hints_checked_star_ {
    use crate::dict::split::hint::*;

    /// Pins the entry count against the live SBCL image
    /// (REPL probe 2026-05-17). A drift here means upstream
    /// `dict-split.lisp` added or removed audited-seq rows
    /// since this port; refresh the literal from a fresh
    /// `(length ichiran/dict::*hints-checked*)` probe.
    #[test]
    fn entry_count_matches_upstream() {
        assert_eq!(HINTS_CHECKED.len(), 162);
    }
}
