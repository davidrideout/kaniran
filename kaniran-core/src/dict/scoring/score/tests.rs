mod length_multiplier {
    use crate::dict::scoring::score::*;

    // Columns: `(length, power, len-lim) -> result`. Covers both branches
    // and the `length == len-lim` boundary.
    #[test]
    fn length_multiplier_fixtures() {
        let cases: &[(i64, i64, i64, i64)] = &[
            // length <= len-lim  → length^power
            (3, 2, 5, 9),
            (5, 2, 5, 25), // boundary: length == len-lim
            (4, 3, 6, 64),
            (3, 1, 5, 3),
            (1, 4, 2, 1),
            // length > len-lim   → length * len-lim^(power-1)
            (7, 2, 5, 35),
            (8, 3, 6, 288),
            (7, 1, 5, 7), // power 1 → len-lim^0 = 1
            (10, 2, 3, 30),
            (6, 4, 4, 384),
        ];
        for &(length, power, len_lim, expected) in cases {
            assert_eq!(
                length_multiplier(length, power, len_lim),
                expected,
                "length={length} power={power} len_lim={len_lim}"
            );
        }
    }
}

mod _star_length_coeff_sequences_star_ {
    use crate::dict::scoring::score::*;

    #[test]
    fn matches_introspected_value() {
        assert_eq!(LENGTH_COEFF_SEQUENCES.len(), 4);
        assert_eq!(
            LENGTH_COEFF_SEQUENCES[0],
            (KaniLengthClass::Strong, &[1i64, 8, 24, 40, 60][..])
        );
        assert_eq!(
            LENGTH_COEFF_SEQUENCES[1],
            (KaniLengthClass::Weak, &[1i64, 4, 9, 16, 25, 36][..])
        );
        assert_eq!(
            LENGTH_COEFF_SEQUENCES[2],
            (KaniLengthClass::Tail, &[4i64, 9, 16, 24][..])
        );
        assert_eq!(
            LENGTH_COEFF_SEQUENCES[3],
            (KaniLengthClass::Ltail, &[4i64, 12, 18, 24][..])
        );
    }
}

mod length_multiplier_coeff {
    use crate::dict::scoring::score::*;

    #[test]
    fn strong_tabled_range() {
        assert_eq!(length_multiplier_coeff(0, KaniLengthClass::Strong), 0);
        assert_eq!(length_multiplier_coeff(1, KaniLengthClass::Strong), 1);
        assert_eq!(length_multiplier_coeff(2, KaniLengthClass::Strong), 8);
        assert_eq!(length_multiplier_coeff(3, KaniLengthClass::Strong), 24);
        assert_eq!(length_multiplier_coeff(4, KaniLengthClass::Strong), 40);
        assert_eq!(length_multiplier_coeff(5, KaniLengthClass::Strong), 60);
    }

    #[test]
    fn strong_extrapolation() {
        // n = 5, last = 60, last/n = 12. length * 12 outside range.
        assert_eq!(length_multiplier_coeff(6, KaniLengthClass::Strong), 72);
        assert_eq!(length_multiplier_coeff(7, KaniLengthClass::Strong), 84);
        assert_eq!(length_multiplier_coeff(8, KaniLengthClass::Strong), 96);
        assert_eq!(length_multiplier_coeff(10, KaniLengthClass::Strong), 120);
        assert_eq!(length_multiplier_coeff(50, KaniLengthClass::Strong), 600);
    }

    #[test]
    fn weak_tabled_range() {
        assert_eq!(length_multiplier_coeff(0, KaniLengthClass::Weak), 0);
        assert_eq!(length_multiplier_coeff(1, KaniLengthClass::Weak), 1);
        assert_eq!(length_multiplier_coeff(2, KaniLengthClass::Weak), 4);
        assert_eq!(length_multiplier_coeff(3, KaniLengthClass::Weak), 9);
        assert_eq!(length_multiplier_coeff(4, KaniLengthClass::Weak), 16);
        assert_eq!(length_multiplier_coeff(5, KaniLengthClass::Weak), 25);
        assert_eq!(length_multiplier_coeff(6, KaniLengthClass::Weak), 36);
    }

    #[test]
    fn weak_extrapolation() {
        // n = 6, last = 36, last/n = 6.
        assert_eq!(length_multiplier_coeff(7, KaniLengthClass::Weak), 42);
        assert_eq!(length_multiplier_coeff(8, KaniLengthClass::Weak), 48);
        assert_eq!(length_multiplier_coeff(100, KaniLengthClass::Weak), 600);
    }

    #[test]
    fn tail_tabled_range() {
        assert_eq!(length_multiplier_coeff(0, KaniLengthClass::Tail), 0);
        assert_eq!(length_multiplier_coeff(1, KaniLengthClass::Tail), 4);
        assert_eq!(length_multiplier_coeff(2, KaniLengthClass::Tail), 9);
        assert_eq!(length_multiplier_coeff(3, KaniLengthClass::Tail), 16);
        assert_eq!(length_multiplier_coeff(4, KaniLengthClass::Tail), 24);
    }

    #[test]
    fn tail_extrapolation() {
        // n = 4, last = 24, last/n = 6.
        assert_eq!(length_multiplier_coeff(5, KaniLengthClass::Tail), 30);
        assert_eq!(length_multiplier_coeff(6, KaniLengthClass::Tail), 36);
        assert_eq!(length_multiplier_coeff(7, KaniLengthClass::Tail), 42);
        assert_eq!(length_multiplier_coeff(8, KaniLengthClass::Tail), 48);
        assert_eq!(length_multiplier_coeff(1000, KaniLengthClass::Tail), 6000);
    }

    #[test]
    fn ltail_tabled_range() {
        assert_eq!(length_multiplier_coeff(0, KaniLengthClass::Ltail), 0);
        assert_eq!(length_multiplier_coeff(1, KaniLengthClass::Ltail), 4);
        assert_eq!(length_multiplier_coeff(2, KaniLengthClass::Ltail), 12);
        assert_eq!(length_multiplier_coeff(3, KaniLengthClass::Ltail), 18);
        assert_eq!(length_multiplier_coeff(4, KaniLengthClass::Ltail), 24);
    }

    #[test]
    fn ltail_extrapolation() {
        // n = 4, last = 24, last/n = 6.
        assert_eq!(length_multiplier_coeff(5, KaniLengthClass::Ltail), 30);
        assert_eq!(length_multiplier_coeff(6, KaniLengthClass::Ltail), 36);
        assert_eq!(length_multiplier_coeff(7, KaniLengthClass::Ltail), 42);
        assert_eq!(length_multiplier_coeff(8, KaniLengthClass::Ltail), 48);
        assert_eq!(
            length_multiplier_coeff(10000, KaniLengthClass::Ltail),
            60000
        );
    }
}

mod kanji_break_penalty {
    use crate::dict::scoring::score::*;

    // ----- pure-arithmetic cases (no info, no DB) -----

    #[test]
    fn no_info_above_cutoff_halves_with_ceiling() {
        // 100 >= cutoff → max(5, ceil(100/2)) = 50
        let ctx = KaniranContext::from_env().expect("ctx");
        let got = kanji_break_penalty(&ctx, &[0], 100, None, "", None, None)
            
            .unwrap();
        assert_eq!(got, 50);
    }

    #[test]
    fn no_info_odd_score_rounds_up() {
        // Same arithmetic; the end branch is unused without posi.
        let ctx = KaniranContext::from_env().expect("ctx");
        let got = kanji_break_penalty(&ctx, &[1], 100, None, "", None, None)
            
            .unwrap();
        assert_eq!(got, 50);
    }

    #[test]
    fn no_info_both_branch() {
        let ctx = KaniranContext::from_env().expect("ctx");
        let got = kanji_break_penalty(&ctx, &[0, 5], 100, None, "", None, None)
            
            .unwrap();
        assert_eq!(got, 50);
    }

    #[test]
    fn below_cutoff_returns_unchanged() {
        // A score below the cutoff (5) is returned unchanged.
        let ctx = KaniranContext::from_env().expect("ctx");
        let got = kanji_break_penalty(&ctx, &[0], 4, None, "", None, None)
            
            .unwrap();
        assert_eq!(got, 4);
    }

    // ----- info-bearing cases (calc_score + kanji_break_penalty integration) -----
    //
    // The pure-arithmetic cases above exercise the no-info arm. These
    // exercise the four branches that gate on info contents.

    /// For 猫 (a common noun), none of the short-circuit conditions apply,
    /// so the penalty falls through and halves the score: 19 → 10.
    #[cfg(feature = "postgres")]
    #[test]
    fn info_fall_through_penalty() {
        use crate::dict::kani_word::KaniWordDispatchEnum;
        use crate::dict::scoring::calc_score::calc_score;
        let ctx = KaniranContext::from_env().expect("ctx");
        let rows: Vec<crate::dict::dao::KanjiText> = tokio::runtime::Runtime::new()
            .expect("tokio runtime")
            .block_on(
                sqlx::query_as(
                    "SELECT * FROM kanji_text WHERE seq = 1467640 AND text = '猫' ORDER BY id LIMIT 1",
                )
                .fetch_all(ctx.pool.as_ref().expect("postgres pool")),
            )
            .expect("猫 1467640 row");
        let w = KaniWordDispatchEnum::Kanji(rows.into_iter().next().unwrap());
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).unwrap();
        assert_eq!(score, 19);
        let info = info.unwrap();
        let got = kanji_break_penalty(&ctx, &[0], score, Some(&info), "猫", None, None)
            
            .unwrap();
        assert_eq!(got, 10);
    }

    /// 飲む is on the no-kanji-break-penalty list, so the score is returned
    /// unchanged regardless of the arithmetic.
    #[cfg(feature = "postgres")]
    #[test]
    fn no_penalty_list_short_circuit() {
        use crate::dict::kani_word::KaniWordDispatchEnum;
        use crate::dict::scoring::calc_score::calc_score;
        let ctx = KaniranContext::from_env().expect("ctx");
        let rows: Vec<crate::dict::dao::KanjiText> = tokio::runtime::Runtime::new()
            .expect("tokio runtime")
            .block_on(
                sqlx::query_as(
                    "SELECT * FROM kanji_text WHERE seq = 1169870 AND text = '飲む' ORDER BY id LIMIT 1",
                )
                .fetch_all(ctx.pool.as_ref().expect("postgres pool")),
            )
            .expect("飲む 1169870 row");
        let w = KaniWordDispatchEnum::Kanji(rows.into_iter().next().unwrap());
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).unwrap();
        let info = info.unwrap();
        let got = kanji_break_penalty(&ctx, &[0], score, Some(&info), "飲む", None, None)
            
            .unwrap();
        assert_eq!(got, score);
    }

    /// 好き is on the no-kanji-break-penalty list, so the score is
    /// unchanged regardless of text. Even without that, its す-prefix would
    /// also short-circuit the penalty.
    #[cfg(feature = "postgres")]
    #[test]
    fn suki_seq_short_circuit() {
        use crate::dict::kani_word::KaniWordDispatchEnum;
        use crate::dict::scoring::calc_score::calc_score;
        let ctx = KaniranContext::from_env().expect("ctx");
        let rows: Vec<crate::dict::dao::KanjiText> = tokio::runtime::Runtime::new()
            .expect("tokio runtime")
            .block_on(
                sqlx::query_as(
                    "SELECT * FROM kanji_text WHERE seq = 1277450 AND text = '好き' ORDER BY id LIMIT 1",
                )
                .fetch_all(ctx.pool.as_ref().expect("postgres pool")),
            )
            .expect("好き 1277450 row");
        let w = KaniWordDispatchEnum::Kanji(rows.into_iter().next().unwrap());
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).unwrap();
        let info = info.unwrap();
        let got_kanji_text =
            kanji_break_penalty(&ctx, &[0], score, Some(&info), "好き", None, None)
                
                .unwrap();
        let got_kana_text = kanji_break_penalty(&ctx, &[0], score, Some(&info), "すき", None, None)
            
            .unwrap();
        // Both leave the score unchanged — the seq-set check fires first.
        assert_eq!(got_kanji_text, score);
        assert_eq!(got_kana_text, score);
    }

    #[test]
    fn classify_end_results() {
        // A multi-element break is Both; a single 0 is Beg; anything else
        // (including empty) is End.
        assert_eq!(classify_end(&[]), KanjiBreakEnd::End);
        assert_eq!(classify_end(&[0]), KanjiBreakEnd::Beg);
        assert_eq!(classify_end(&[3]), KanjiBreakEnd::End);
        assert_eq!(classify_end(&[5]), KanjiBreakEnd::End);
        assert_eq!(classify_end(&[0, 2]), KanjiBreakEnd::Both);
        assert_eq!(classify_end(&[1, 4]), KanjiBreakEnd::Both);
        assert_eq!(classify_end(&[0, 1, 2]), KanjiBreakEnd::Both);
    }
}

mod get_non_arch_posi {
    use crate::conn::kani_context::KaniranContext;
    use crate::dict::scoring::score::*;

    // Each test sorts the result before comparing: the query has no ORDER
    // BY, so Postgres may return the distinct rows in any order.
    fn sorted(mut v: Vec<String>) -> Vec<String> {
        v.sort();
        v
    }

    #[test]
    fn taberu_single_seq() {
        let ctx = KaniranContext::from_env().expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400]).expect("query");
        assert_eq!(sorted(got), vec!["v5m".to_string(), "vt".to_string()]);
    }

    #[test]
    fn no_particle_seq() {
        let ctx = KaniranContext::from_env().expect("ctx");
        let got = get_non_arch_posi(&ctx, &[2089020]).expect("query");
        assert_eq!(
            sorted(got),
            vec!["aux-v".to_string(), "cop".to_string(), "cop-da".to_string(),]
        );
    }

    #[test]
    fn dummy_seq_1000220() {
        let ctx = KaniranContext::from_env().expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1000220]).expect("query");
        assert_eq!(sorted(got), vec!["adj-na".to_string()]);
    }

    #[test]
    fn hon_noun_seq() {
        let ctx = KaniranContext::from_env().expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1522150]).expect("query");
        assert_eq!(
            sorted(got),
            vec!["ctr".to_string(), "n".to_string(), "pref".to_string()]
        );
    }

    #[test]
    fn counter_seq_1325880() {
        let ctx = KaniranContext::from_env().expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1325880]).expect("query");
        assert_eq!(sorted(got), vec!["n".to_string()]);
    }

    #[test]
    fn two_seqs_union() {
        let ctx = KaniranContext::from_env().expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400, 2089020])
            
            .expect("query");
        assert_eq!(
            sorted(got),
            vec![
                "aux-v".to_string(),
                "cop".to_string(),
                "cop-da".to_string(),
                "v5m".to_string(),
                "vt".to_string(),
            ]
        );
    }

    #[test]
    fn zo_particle_seq() {
        let ctx = KaniranContext::from_env().expect("ctx");
        let got = get_non_arch_posi(&ctx, &[2029110]).expect("query");
        assert_eq!(sorted(got), vec!["int".to_string(), "prt".to_string()]);
    }

    #[test]
    fn unknown_seq_returns_empty() {
        let ctx = KaniranContext::from_env().expect("ctx");
        let got = get_non_arch_posi(&ctx, &[99999999]).expect("query");
        assert!(got.is_empty(), "expected NIL, got {got:?}");
    }

    #[test]
    fn empty_seq_set_returns_empty() {
        let ctx = KaniranContext::from_env().expect("ctx");
        let got = get_non_arch_posi(&ctx, &[]).expect("query");
        assert!(got.is_empty(), "expected NIL, got {got:?}");
    }

    #[test]
    fn many_seqs_union() {
        let ctx = KaniranContext::from_env().expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400, 2089020, 1522150, 1000220])
            
            .expect("query");
        assert_eq!(
            sorted(got),
            vec![
                "adj-na".to_string(),
                "aux-v".to_string(),
                "cop".to_string(),
                "cop-da".to_string(),
                "ctr".to_string(),
                "n".to_string(),
                "pref".to_string(),
                "v5m".to_string(),
                "vt".to_string(),
            ]
        );
    }

    #[test]
    fn taberu_with_conj_root() {
        let ctx = KaniranContext::from_env().expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400, 2027820])
            
            .expect("query");
        assert_eq!(
            sorted(got),
            vec!["exp".to_string(), "v5m".to_string(), "vt".to_string()]
        );
    }
}

mod gen_score {
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::readings::{find_word, FindWordRows};
    use crate::dict::scoring::score::*;
    use crate::dict::scoring::score::{KaniSplitInfo, Segment};

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn first_kana_for(ctx: &KaniranContext, s: &str) -> KaniWordDispatchEnum {
        match find_word(ctx, s, false).unwrap().into_owned() {
            FindWordRows::Kana(mut v) => KaniWordDispatchEnum::Kana(v.remove(0)),
            FindWordRows::Kanji(mut v) => KaniWordDispatchEnum::Kanji(v.remove(0)),
        }
    }

    /// Deterministic single-row fetch — `find-word`'s SQL has no
    /// ORDER BY, so the same lookup can rotate first rows between
    /// runs / databases.
    #[cfg(feature = "postgres")]
    fn kana_by_seq_text(ctx: &KaniranContext, seq: i32, text: &str) -> KaniWordDispatchEnum {
        let rows: Vec<crate::dict::dao::KanaText> = tokio::runtime::Runtime::new()
            .expect("tokio runtime")
            .block_on(
                sqlx::query_as(
                    "SELECT * FROM kana_text WHERE seq = $1 AND text = $2 ORDER BY id LIMIT 1",
                )
                .bind(seq)
                .bind(text)
                .fetch_all(ctx.pool.as_ref().expect("postgres pool")),
            )
            .expect("query");
        KaniWordDispatchEnum::Kana(rows.into_iter().next().expect("row exists"))
    }

    #[cfg(feature = "postgres")]
    fn kanji_by_seq_text(ctx: &KaniranContext, seq: i32, text: &str) -> KaniWordDispatchEnum {
        let rows: Vec<crate::dict::dao::KanjiText> = tokio::runtime::Runtime::new()
            .expect("tokio runtime")
            .block_on(
                sqlx::query_as(
                    "SELECT * FROM kanji_text WHERE seq = $1 AND text = $2 ORDER BY id LIMIT 1",
                )
                .bind(seq)
                .bind(text)
                .fetch_all(ctx.pool.as_ref().expect("postgres pool")),
            )
            .expect("query");
        KaniWordDispatchEnum::Kanji(rows.into_iter().next().expect("row exists"))
    }

    fn make_segment(word: KaniWordDispatchEnum, end: usize, text: &str) -> Segment {
        Segment {
            start: 0,
            end,
            word,
            score: None,
            info: None,
            top: None,
            text: Some(text.to_string()),
        }
    }

    /// Scoring a ねこ segment writes both its score and its info.
    #[test]
    fn neko_baseline_writes_score_and_info() {
        let ctx = ctx_from_env();
        let w = first_kana_for(&ctx, "ねこ");
        let mut seg = make_segment(w, 2, "ねこ");
        gen_score(&ctx, &mut seg, false, &[]).unwrap();
        assert_eq!(seg.score, Some(16));
        let info = seg.info.as_ref().unwrap();
        assert_eq!(info.posi, vec!["n".to_string()]);
        assert_eq!(info.seq_set, vec![1467640]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(7));
        assert_eq!(info.score_info.prop_score, 4);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, false, true, false));
    }

    /// A kanji-break passed to gen_score propagates through calc_score
    /// into the segment's info. Uses a deterministic-row helper to avoid
    /// the unordered find-word lookup.
    #[cfg(feature = "postgres")]
    #[test]
    fn neko_kanji_break_propagates_through_calc_score() {
        let ctx = ctx_from_env();
        let w = kanji_by_seq_text(&ctx, 2698030, "猫");
        let mut seg = make_segment(w, 1, "猫");
        gen_score(&ctx, &mut seg, false, &[0]).unwrap();
        assert_eq!(seg.score, Some(3));
        let info = seg.info.as_ref().unwrap();
        assert!(info.posi.is_empty());
        assert_eq!(info.seq_set, vec![2698030]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, None);
        assert_eq!(info.score_info.prop_score, 3);
        assert_eq!(info.score_info.kanji_break, vec![0]);
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (true, false, false, false));
    }

    /// A common noun reading of ね scored in final position.
    #[cfg(feature = "postgres")]
    #[test]
    fn ne_final_common_n_branch() {
        let ctx = ctx_from_env();
        let w = kana_by_seq_text(&ctx, 1290020, "ね");
        let mut seg = make_segment(w, 1, "ね");
        gen_score(&ctx, &mut seg, true, &[]).unwrap();
        assert_eq!(seg.score, Some(4));
        let info = seg.info.as_ref().unwrap();
        assert_eq!(info.posi, vec!["n".to_string()]);
        assert_eq!(info.seq_set, vec![1290020]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(5));
        assert_eq!(info.score_info.prop_score, 4);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, false, true, false));
    }
}

mod find_sticky_positions {
    use crate::dict::scoring::score::find_sticky_positions;

    #[test]
    fn empty_string() {
        assert_eq!(find_sticky_positions(""), Vec::<usize>::new());
    }

    #[test]
    fn no_stickies_kanji() {
        assert_eq!(find_sticky_positions("食べる"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("学校"), Vec::<usize>::new());
        assert_eq!(
            find_sticky_positions("私はその本を読みました"),
            Vec::<usize>::new()
        );
        assert_eq!(find_sticky_positions("東京特許許可局"), Vec::<usize>::new());
    }

    #[test]
    fn modifier_mid_word() {
        assert_eq!(find_sticky_positions("きゃく"), vec![1]);
        assert_eq!(find_sticky_positions("けーき"), vec![1]);
        assert_eq!(find_sticky_positions("あぁい"), vec![1]);
    }

    #[test]
    fn modifier_at_end_collected_when_no_long_vowel_match() {
        // +YA at end: long_vowel_modifier_p returns false (not in +A/+I/+U/+E/+O).
        assert_eq!(find_sticky_positions("きゃ"), vec![1]);
        // +A after KI: vowels don't agree (KI ends in I), so collected.
        assert_eq!(find_sticky_positions("きぁ"), vec![1]);
        // +O after NI: vowels don't agree, collected.
        assert_eq!(find_sticky_positions("にぉ"), vec![1]);
        // Modifier after non-kana char (prev has no KanaClass): collected.
        assert_eq!(find_sticky_positions("漢ぁ"), vec![1]);
        // +WA at end: long_vowel_modifier_p false for PlusWa, collected.
        assert_eq!(find_sticky_positions("かゎ"), vec![1]);
    }

    #[test]
    fn modifier_at_end_suppressed_when_long_vowel_matches() {
        // +A after KA: vowel agrees, suppressed.
        assert_eq!(find_sticky_positions("かぁ"), Vec::<usize>::new());
        // +I after NI: vowel agrees, suppressed.
        assert_eq!(find_sticky_positions("にぃ"), Vec::<usize>::new());
    }

    #[test]
    fn long_vowel_at_end_suppressed() {
        assert_eq!(find_sticky_positions("かー"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("あー"), Vec::<usize>::new());
    }

    #[test]
    fn long_vowel_at_start_collected() {
        assert_eq!(find_sticky_positions("ーあ"), vec![0]);
    }

    #[test]
    fn modifier_first_char_not_last_collected() {
        // Modifier at pos 0 with str_len > 1: not last, so lvmp branch irrelevant.
        assert_eq!(find_sticky_positions("ぁか"), vec![0]);
    }

    #[test]
    fn modifier_lone_char_collected() {
        // pos==0, last, but `(> pos 0)` is false, so lvmp branch short-circuits.
        assert_eq!(find_sticky_positions("ぁ"), vec![0]);
        // Same — PlusWa at lone position.
        assert_eq!(find_sticky_positions("ゎ"), vec![0]);
    }

    #[test]
    fn sokuon_mid_word_collects_pos_plus_one() {
        assert_eq!(find_sticky_positions("いっぱい"), vec![2]);
        assert_eq!(find_sticky_positions("ニッポン"), vec![2]);
        assert_eq!(find_sticky_positions("ニッキ"), vec![2]);
        assert_eq!(find_sticky_positions("っあっい"), vec![1, 3]);
    }

    #[test]
    fn sokuon_at_end_not_collected() {
        assert_eq!(find_sticky_positions("いっ"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("っ"), Vec::<usize>::new());
    }

    #[test]
    fn sokuon_followed_by_non_kana_not_collected() {
        assert_eq!(find_sticky_positions("っ漢"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("っX"), Vec::<usize>::new());
    }

    #[test]
    fn iteration_characters() {
        // Both iter marks: pos 0 not last (collect 0), pos 1 last & not long-vowel & lvmp false → collect 1.
        assert_eq!(find_sticky_positions("ゝゞ"), vec![0, 1]);
        // ゝ at end after い: lvmp false, long-vowel false → collected.
        assert_eq!(find_sticky_positions("いゝ"), vec![1]);
    }

    #[test]
    fn single_kana_char_no_sticky() {
        assert_eq!(find_sticky_positions("あ"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("いろは"), Vec::<usize>::new());
    }

    #[test]
    fn combined_sokuon_and_modifier() {
        assert_eq!(find_sticky_positions("きゃっき"), vec![1, 3]);
    }
}

mod make_slice {
    use crate::dict::scoring::score::*;

    #[test]
    fn empty_seed() {
        let s = make_slice();
        assert_eq!(s.len(), 0);
        assert_eq!(s, "");
    }
}

mod subseq_slice {
    use crate::dict::scoring::score::*;

    /// Offsets count characters, not bytes, across multi-byte UTF-8:
    /// positions 1..3 of "あいうえお" give "いう".
    #[test]
    fn character_offsets_multi_byte() {
        let r = subseq_slice(None, "あいうえお", 1, Some(3));
        assert_eq!(r, "いう");
    }

    #[test]
    fn full_range_ascii() {
        let r = subseq_slice(None, "abcde", 0, Some(5));
        assert_eq!(r, "abcde");
    }

    /// A missing end defaults to the string length.
    #[test]
    fn end_defaults_to_length() {
        let r = subseq_slice(None, "abcde", 0, None);
        assert_eq!(r, "abcde");
    }

    /// With an offset start and default end, the slice runs to the end.
    #[test]
    fn end_default_with_offset_start() {
        let r = subseq_slice(None, "abc", 1, None);
        assert_eq!(r, "bc");
    }

    /// Equal start and end give an empty slice.
    #[test]
    fn empty_range_when_start_equals_end() {
        let r = subseq_slice(None, "hello", 2, Some(2));
        assert_eq!(r, "");
    }

    /// The passed-in slice argument is ignored; the result is a view of
    /// the source string.
    #[test]
    fn slice_argument_is_ignored() {
        let seed = crate::dict::scoring::score::make_slice();
        let r = subseq_slice(Some(seed), "hello", 1, Some(4));
        assert_eq!(r, "ell");
    }

    /// An end less than the start panics.
    #[test]
    #[should_panic(expected = "subseq-slice: end (2) < start (4)")]
    fn end_less_than_start_panics() {
        let _ = subseq_slice(None, "hello", 4, Some(2));
    }

    /// An end past the string length panics.
    #[test]
    #[should_panic(expected = "subseq-slice: end (10) > (length s) (5)")]
    fn end_past_length_panics() {
        let _ = subseq_slice(None, "hello", 0, Some(10));
    }

    /// An in-range start with an end past the length panics.
    #[test]
    #[should_panic(expected = "subseq-slice: end (7) > (length s) (5)")]
    fn start_in_range_end_past_length_panics() {
        let _ = subseq_slice(None, "hello", 2, Some(7));
    }

    /// A start and end both past the length panic via the end-bound check.
    #[test]
    #[should_panic(expected = "subseq-slice: end (12) > (length s) (5)")]
    fn start_and_end_past_length_panics() {
        let _ = subseq_slice(None, "hello", 10, Some(12));
    }

    /// Start == end == length is the allowed upper edge (empty result, no
    /// panic).
    #[test]
    fn start_equal_to_length_at_end_is_ok() {
        let r = subseq_slice(None, "hello", 5, Some(5));
        assert_eq!(r, "");
    }

    /// An end equal to the length is the allowed upper edge.
    #[test]
    fn end_equal_to_length_is_ok() {
        let r = subseq_slice(None, "hello", 0, Some(5));
        assert_eq!(r, "hello");
    }
}

mod compare_common {
    use crate::dict::scoring::score::*;
    use CompareCommonResult::*;

    // The first branch returns c1 itself (e.g. compare(5, none) = C1(5));
    // the second and third return True or Nil.
    #[test]
    fn nil_c1_always_nil() {
        for c2 in [None, Some(0), Some(1), Some(2), Some(5), Some(10), Some(-3)] {
            assert_eq!(compare_common(None, c2), Nil);
        }
    }

    #[test]
    fn nil_c2_returns_c1_itself() {
        assert_eq!(compare_common(Some(0), None), C1(0));
        assert_eq!(compare_common(Some(1), None), C1(1));
        assert_eq!(compare_common(Some(2), None), C1(2));
        assert_eq!(compare_common(Some(5), None), C1(5));
        assert_eq!(compare_common(Some(10), None), C1(10));
        assert_eq!(compare_common(Some(-3), None), C1(-3));
    }

    #[test]
    fn zero_c1_only_truthy_when_c2_nil() {
        // A zero c1 against an absent c2 is still truthy; every other c2 is Nil.
        assert_eq!(compare_common(Some(0), None), C1(0));
        assert_eq!(compare_common(Some(0), Some(0)), Nil);
        assert_eq!(compare_common(Some(0), Some(1)), Nil);
        assert_eq!(compare_common(Some(0), Some(2)), Nil);
        assert_eq!(compare_common(Some(0), Some(5)), Nil);
        assert_eq!(compare_common(Some(0), Some(10)), Nil);
        assert_eq!(compare_common(Some(0), Some(-3)), Nil);
    }

    #[test]
    fn c2_zero_returns_true_when_c1_positive() {
        assert_eq!(compare_common(Some(1), Some(0)), True);
        assert_eq!(compare_common(Some(2), Some(0)), True);
        assert_eq!(compare_common(Some(5), Some(0)), True);
        assert_eq!(compare_common(Some(10), Some(0)), True);
        assert_eq!(compare_common(Some(0), Some(0)), Nil);
        assert_eq!(compare_common(Some(-3), Some(0)), Nil);
    }

    #[test]
    fn positive_pair_lt_predicate() {
        // Two positive values compare by less-than.
        assert_eq!(compare_common(Some(1), Some(2)), True);
        assert_eq!(compare_common(Some(1), Some(5)), True);
        assert_eq!(compare_common(Some(1), Some(10)), True);
        assert_eq!(compare_common(Some(2), Some(5)), True);
        assert_eq!(compare_common(Some(2), Some(10)), True);
        assert_eq!(compare_common(Some(5), Some(10)), True);
        assert_eq!(compare_common(Some(1), Some(1)), Nil);
        assert_eq!(compare_common(Some(2), Some(1)), Nil);
        assert_eq!(compare_common(Some(2), Some(2)), Nil);
        assert_eq!(compare_common(Some(5), Some(1)), Nil);
        assert_eq!(compare_common(Some(5), Some(2)), Nil);
        assert_eq!(compare_common(Some(5), Some(5)), Nil);
        assert_eq!(compare_common(Some(10), Some(1)), Nil);
        assert_eq!(compare_common(Some(10), Some(5)), Nil);
        assert_eq!(compare_common(Some(10), Some(10)), Nil);
    }

    #[test]
    fn negative_c1_falls_off() {
        // A negative c1 is never less than a positive c2, so it falls off to Nil.
        assert_eq!(compare_common(Some(-3), Some(1)), Nil);
        assert_eq!(compare_common(Some(-3), Some(2)), Nil);
        assert_eq!(compare_common(Some(-3), Some(5)), Nil);
        assert_eq!(compare_common(Some(-3), Some(10)), Nil);
        assert_eq!(compare_common(Some(-3), Some(-3)), Nil);
        // Any positive c1 against c2 = -3 is also Nil (c1 is not less than -3).
        assert_eq!(compare_common(Some(1), Some(-3)), Nil);
        assert_eq!(compare_common(Some(2), Some(-3)), Nil);
        assert_eq!(compare_common(Some(5), Some(-3)), Nil);
        assert_eq!(compare_common(Some(10), Some(-3)), Nil);
    }

    #[test]
    fn is_truthy_maps_nil_to_false() {
        assert!(!Nil.is_truthy());
        assert!(C1(0).is_truthy());
        assert!(C1(-3).is_truthy());
        assert!(C1(5).is_truthy());
        assert!(True.is_truthy());
    }
}

mod cull_segments {
    use crate::dict::dao::KanaText;
    use crate::dict::dao::SimpleText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::scoring::score::*;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo};

    fn dummy_word(seq: i32) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn info_with_common(common: Option<i32>) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: Vec::new(),
            seq_set: Vec::new(),
            conj: Vec::new(),
            common,
            score_info: KaniScoreInfo {
                prop_score: 0,
                kanji_break: Vec::new(),
                use_length_bonus: 0,
                split_info: KaniSplitInfo::None,
            },
            kpcl: (false, false, false, false),
        }
    }

    fn seg(seq: i32, score: i32, common: Option<Option<i32>>) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(seq),
            score: Some(score),
            info: common.map(info_with_common),
            top: None,
            text: None,
        }
    }

    fn scores(segs: &[Segment]) -> Vec<i32> {
        segs.iter().map(|s| s.score.unwrap()).collect()
    }

    fn seqs(segs: &[Segment]) -> Vec<i32> {
        segs.iter()
            .map(|s| match &s.word {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => unreachable!(),
            })
            .collect()
    }

    #[test]
    fn empty_input_returns_empty() {
        let out = cull_segments(Vec::new());
        assert!(out.is_empty());
    }

    #[test]
    fn single_segment_passes_through() {
        let out = cull_segments(vec![seg(1, 10, None)]);
        assert_eq!(scores(&out), vec![10]);
        assert_eq!(seqs(&out), vec![1]);
    }

    // Scores [20, 15, 9, 8]: max=20, cutoff=10, so [20, 15] survive.
    #[test]
    fn descending_scores_cull_below_half() {
        let out = cull_segments(vec![
            seg(1, 20, None),
            seg(2, 15, None),
            seg(3, 9, None),
            seg(4, 8, None),
        ]);
        assert_eq!(scores(&out), vec![20, 15]);
        assert_eq!(seqs(&out), vec![1, 2]);
    }

    // Identical scores: none culled, input order preserved.
    #[test]
    fn identical_scores_none_culled() {
        let out = cull_segments(vec![seg(1, 10, None), seg(2, 10, None), seg(3, 10, None)]);
        assert_eq!(scores(&out), vec![10, 10, 10]);
        assert_eq!(seqs(&out), vec![1, 2, 3]);
    }

    // Unsorted [5, 20, 15, 12] sorts descending; max=20, cutoff=10, so
    // [20, 15, 12] survive.
    #[test]
    fn unsorted_input_sorted_descending() {
        let out = cull_segments(vec![
            seg(1, 5, None),
            seg(2, 20, None),
            seg(3, 15, None),
            seg(4, 12, None),
        ]);
        assert_eq!(scores(&out), vec![20, 15, 12]);
        assert_eq!(seqs(&out), vec![2, 3, 4]);
    }

    // Equal scores, varying commons [nil, 0, 10, 5]: common is the primary
    // sort key, so the order becomes commons [5, 10, 0, nil].
    #[test]
    fn same_score_varying_commons() {
        let out = cull_segments(vec![
            seg(1, 10, Some(None)),
            seg(2, 10, Some(Some(0))),
            seg(3, 10, Some(Some(10))),
            seg(4, 10, Some(Some(5))),
        ]);
        assert_eq!(scores(&out), vec![10, 10, 10, 10]);
        // Order commons [5, 10, 0, nil] maps to seqs [4, 3, 2, 1].
        assert_eq!(seqs(&out), vec![4, 3, 2, 1]);
    }

    // Boundary [10, 5, 4]: cutoff=5, so 5 stays (>= cutoff) and 4 drops.
    #[test]
    fn boundary_cutoff_equal_kept() {
        let out = cull_segments(vec![seg(1, 10, None), seg(2, 5, None), seg(3, 4, None)]);
        assert_eq!(scores(&out), vec![10, 5]);
    }

    // Odd boundary [11, 6, 5]: cutoff=5.5, so 6 stays and 5 drops.
    #[test]
    fn odd_boundary_cutoff_strict() {
        let out = cull_segments(vec![seg(1, 11, None), seg(2, 6, None), seg(3, 5, None)]);
        assert_eq!(scores(&out), vec![11, 6]);
    }

    // Odd boundary [11, 5]: cutoff=5.5, so 5 drops.
    #[test]
    fn odd_boundary_drops_below_half() {
        let out = cull_segments(vec![seg(1, 11, None), seg(2, 5, None)]);
        assert_eq!(scores(&out), vec![11]);
    }

    // Even boundary [10, 5]: cutoff=5, so 5 is kept.
    #[test]
    fn even_boundary_keeps_exactly_half() {
        let out = cull_segments(vec![seg(1, 10, None), seg(2, 5, None)]);
        assert_eq!(scores(&out), vec![10, 5]);
    }

    // Zero scores: cutoff=0, all kept.
    #[test]
    fn zero_scores_all_kept() {
        let out = cull_segments(vec![seg(1, 0, None), seg(2, 0, None)]);
        assert_eq!(scores(&out), vec![0, 0]);
    }

    // Negative scores [-10, -5]: max=-5, cutoff=-2.5, so even the top
    // segment is below cutoff and everything is dropped.
    #[test]
    fn negative_scores_all_culled() {
        let out = cull_segments(vec![seg(1, -10, None), seg(2, -5, None)]);
        assert!(out.is_empty());
    }

    // Common-ordering on [nil, 5, 0, 3] with equal scores gives order
    // [3, 5, 0, nil], exercising every comparison branch:
    //   - 3 sorts before 5
    //   - 5 sorts before 0
    //   - 0 sorts before nil
    //   - nil never sorts before anything.
    #[test]
    fn compare_common_ordering_full() {
        let out = cull_segments(vec![
            seg(1, 10, Some(None)),
            seg(2, 10, Some(Some(5))),
            seg(3, 10, Some(Some(0))),
            seg(4, 10, Some(Some(3))),
        ]);
        // Order commons [3, 5, 0, nil] maps to seqs [4, 2, 3, 1].
        assert_eq!(seqs(&out), vec![4, 2, 3, 1]);
    }
}
