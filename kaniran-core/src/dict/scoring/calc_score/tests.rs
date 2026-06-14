mod calc_score {
    use crate::dict::dao::KanaText;
    use crate::dict::dao::SimpleText;
    use crate::dict::readings::{find_word, FindWordRows};
    use crate::dict::scoring::calc_score::*;
    use crate::dict::scoring::score::KaniSplitInfo;
    use crate::dict::text_classes::CompoundText;

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn first_word_for(ctx: &KaniranContext, s: &str) -> KaniWordDispatchEnum {
        match find_word(ctx, s, false).unwrap().into_owned() {
            FindWordRows::Kana(mut v) => KaniWordDispatchEnum::Kana(v.remove(0)),
            FindWordRows::Kanji(mut v) => KaniWordDispatchEnum::Kanji(v.remove(0)),
        }
    }

    /// Fetch a specific kana_text row by (seq, text) — deterministic
    /// alternative to `first_word_for` when find-word's row order
    /// (no upstream ORDER BY) would make a test flaky.
    #[cfg(feature = "postgres")]
    fn kana_by_seq_text(ctx: &KaniranContext, seq: i32, text: &str) -> KaniWordDispatchEnum {
        let rows: Vec<crate::dict::dao::KanaText> = sqlx::query_as(
            "SELECT * FROM kana_text WHERE seq = $1 AND text = $2 ORDER BY id LIMIT 1",
        )
        .bind(seq)
        .bind(text)
        .fetch_all(&ctx.pool)
        
        .expect("query");
        KaniWordDispatchEnum::Kana(rows.into_iter().next().expect("row exists"))
    }

    /// Baseline score for the common noun ねこ.
    #[test]
    fn nekko_baseline() {
        let ctx = ctx_from_env();
        let w = first_word_for(&ctx, "ねこ");
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).unwrap();
        assert_eq!(score, 16);
        let info = info.unwrap();
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

    /// The particle の scored in non-final position.
    #[test]
    fn no_particle_non_final() {
        let ctx = ctx_from_env();
        let w = first_word_for(&ctx, "の");
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).unwrap();
        assert_eq!(score, 11);
        let info = info.unwrap();
        assert_eq!(info.posi, vec!["prt".to_string()]);
        assert_eq!(info.seq_set, vec![1469800]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 11);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, false));
    }

    /// The particle の in final position gets the final-particle bonus.
    #[test]
    fn no_particle_final_branch_bonus() {
        let ctx = ctx_from_env();
        let w = first_word_for(&ctx, "の");
        let (score, info) = calc_score(&ctx, &w, true, None, None, &[]).unwrap();
        assert_eq!(score, 16);
        let info = info.unwrap();
        assert_eq!(info.posi, vec!["prt".to_string()]);
        assert_eq!(info.seq_set, vec![1469800]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 16);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, false));
    }

    /// An uncommon noun reading of は (common rank absent).
    #[test]
    fn ha_n_uncommon() {
        let ctx = ctx_from_env();
        let w = first_word_for(&ctx, "は");
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).unwrap();
        assert_eq!(score, 1);
        let info = info.unwrap();
        assert_eq!(info.posi, vec!["n".to_string()]);
        assert_eq!(info.seq_set, vec![1171680]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, None);
        assert_eq!(info.score_info.prop_score, 1);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, false, false, false));
    }

    /// The copula reading of だ (seq 2089020). Uses the explicit-seq
    /// helper because looking up だ returns 6 candidates in undefined
    /// order, which would otherwise alternate between the noun reading and
    /// the copula. Exercises the copula branch and the common=0 arm of the
    /// common-bonus cascade.
    #[cfg(feature = "postgres")]
    #[test]
    fn da_copula_cop_da_p_branch() {
        let ctx = ctx_from_env();
        let w = kana_by_seq_text(&ctx, 2089020, "だ");
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).unwrap();
        assert_eq!(score, 16);
        let info = info.unwrap();
        let mut posi_sorted = info.posi.clone();
        posi_sorted.sort();
        assert_eq!(
            posi_sorted,
            vec!["aux-v".to_string(), "cop".to_string(), "cop-da".to_string()]
        );
        assert_eq!(info.seq_set, vec![2089020]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 16);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, false));

        // :final t produces the same value — particle_p is false (no "prt" in posi),
        // so the final-particle bonus block is skipped.
        let (score, _) = calc_score(&ctx, &w, true, None, None, &[]).unwrap();
        assert_eq!(score, 16);
    }

    /// The root verb 食べる.
    #[test]
    fn taberu_root_verb() {
        let ctx = ctx_from_env();
        let w = first_word_for(&ctx, "食べる");
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).unwrap();
        assert_eq!(score, 504);
        let info = info.unwrap();
        let mut p = info.posi.clone();
        p.sort();
        assert_eq!(p, vec!["v1".to_string(), "vt".to_string()]);
        assert_eq!(info.seq_set, vec![1358280]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(25));
        assert_eq!(info.score_info.prop_score, 21);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (true, true, true, true));
    }

    /// A kanji-break only adjusts the outer score; it does not mutate any
    /// field of `info`, so every non-score-info field matches
    /// [`taberu_root_verb`].
    #[test]
    fn taberu_with_kanji_break() {
        let ctx = ctx_from_env();
        let w = first_word_for(&ctx, "食べる");
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[0]).unwrap();
        assert_eq!(score, 252);
        let info = info.unwrap();
        let mut p = info.posi.clone();
        p.sort();
        assert_eq!(p, vec!["v1".to_string(), "vt".to_string()]);
        assert_eq!(info.seq_set, vec![1358280]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(25));
        assert_eq!(info.score_info.prop_score, 21);
        assert_eq!(info.score_info.kanji_break, vec![0]);
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (true, true, true, true));
    }

    /// A long interjection (ありがとう).
    #[test]
    fn arigatou_interjection_long() {
        let ctx = ctx_from_env();
        let w = first_word_for(&ctx, "ありがとう");
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).unwrap();
        assert_eq!(score, 525);
        let info = info.unwrap();
        assert_eq!(info.posi, vec!["int".to_string()]);
        assert_eq!(info.seq_set, vec![1586820]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 21);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, true));
    }

    /// A long katakana noun (コンピューター).
    #[test]
    fn computer_katakana_path() {
        let ctx = ctx_from_env();
        let w = first_word_for(&ctx, "コンピューター");
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).unwrap();
        assert_eq!(score, 440);
        let info = info.unwrap();
        assert_eq!(info.posi, vec!["n".to_string()]);
        assert_eq!(info.seq_set, vec![1053350]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 11);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        // kpcl: katakana-p=T, primary-p=NIL, common-p=T, long-p=T
        assert_eq!(info.kpcl, (true, false, true, true));
    }

    /// How the use-length bonus scales the score for ねこ at lengths 5, 3,
    /// and 2 (only the score and length bonus change).
    #[test]
    fn neko_use_length_variations() {
        let ctx = ctx_from_env();
        let w = first_word_for(&ctx, "ねこ");

        // Helper to assert every field other than score / use_length_bonus,
        // which are the only quantities that change with use_length.
        let assert_neko_baseline = |info: &KaniSegmentInfo| {
            assert_eq!(info.posi, vec!["n".to_string()]);
            assert_eq!(info.seq_set, vec![1467640]);
            assert!(info.conj.is_empty());
            assert_eq!(info.common, Some(7));
            assert_eq!(info.score_info.prop_score, 4);
            assert!(info.score_info.kanji_break.is_empty());
            assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
            assert_eq!(info.kpcl, (false, false, true, false));
        };

        let (score, info) = calc_score(&ctx, &w, false, Some(5), None, &[])
            
            .unwrap();
        assert_eq!(score, 80);
        let info = info.unwrap();
        assert_neko_baseline(&info);
        assert_eq!(info.score_info.use_length_bonus, 64);

        let (score, info) = calc_score(&ctx, &w, false, Some(3), None, &[])
            
            .unwrap();
        assert_eq!(score, 32);
        let info = info.unwrap();
        assert_neko_baseline(&info);
        assert_eq!(info.score_info.use_length_bonus, 16);

        let (score, info) = calc_score(&ctx, &w, false, Some(2), None, &[])
            
            .unwrap();
        assert_eq!(score, 16);
        let info = info.unwrap();
        assert_neko_baseline(&info);
        assert_eq!(info.score_info.use_length_bonus, 0);
    }

    // A compound-text whose inner score falls on a skip-path returns 0,
    // but the result must still carry a partial info that holds only the
    // conjugation data — not an absent info.

    /// Compound `れちゃう` with no score-base (falls back to its primary).
    /// The last word ちゃう has no conjugation data on the real DB, so the
    /// synthesized info carries an empty conj list.
    #[test]
    fn compound_skipword_partial_info_conj_null() {
        let ctx = ctx_from_env();

        let primary = KanaText {
            id: 532088,
            seq: 10230770,
            text: "れて".into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText {
                conjugations: Some(WordConjugations::Ids(vec![232360])),
                hintedp: false,
            },
        };
        let tail = KanaText {
            id: 108760,
            seq: 2013800,
            text: "ちゃう".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText {
                conjugations: Some(WordConjugations::Root),
                hintedp: false,
            },
        };
        let compound = CompoundText {
            text: "れちゃう".into(),
            kana: "れちゃう".into(),
            primary: Box::new(KaniWordDispatchEnum::Kana(primary.clone())),
            words: vec![
                KaniWordDispatchEnum::Kana(primary),
                KaniWordDispatchEnum::Kana(tail),
            ],
            score_base: None,
            score_mod: ScoreMod::Single(5),
        };
        let word = KaniWordDispatchEnum::Compound(compound);

        let (score, info) = calc_score(&ctx, &word, false, None, None, &[])
            
            .unwrap();
        assert_eq!(score, 0);
        let info = info.expect(
            "compound + skip-word inner must synthesize partial info \
             (mirrors upstream `(setf (getf nil :conj) X)`)",
        );
        assert!(
            info.conj.is_empty(),
            "info.conj: expected empty (last word ちゃう :ROOT has no conj-data); got {:?}",
            info.conj,
        );
        // The other five fields default to zero/empty — only the
        // conjugation field is synthesized.
        assert!(info.posi.is_empty(), "info.posi: {:?}", info.posi);
        assert!(info.seq_set.is_empty(), "info.seq_set: {:?}", info.seq_set);
        assert_eq!(info.common, None);
        assert_eq!(info.score_info.prop_score, 0);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, false, false, false));
    }

    /// Compound `られなくなりました` with no score-base. Its last word
    /// なりました does have conjugation data (from なる), so the
    /// synthesized info carries a single conj-data entry.
    #[test]
    fn compound_skipword_partial_info_conj_non_null() {
        let ctx = ctx_from_env();

        let rare = KanaText {
            id: 0,
            seq: 10230810,
            text: "られ".into(),
            ord: 1,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText {
                conjugations: Some(WordConjugations::Ids(vec![232400])),
                hintedp: false,
            },
        };
        let naku = KanaText {
            id: 1030305,
            seq: 10648808,
            text: "なく".into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText {
                conjugations: Some(WordConjugations::Ids(vec![656991])),
                hintedp: false,
            },
        };
        let narimashita = KanaText {
            id: 704041,
            seq: 10374833,
            text: "なりました".into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText {
                conjugations: Some(WordConjugations::Ids(vec![378046])),
                hintedp: false,
            },
        };
        let compound = CompoundText {
            text: "られなくなりました".into(),
            kana: "られなくなりました".into(),
            primary: Box::new(KaniWordDispatchEnum::Kana(rare.clone())),
            words: vec![
                KaniWordDispatchEnum::Kana(rare),
                KaniWordDispatchEnum::Kana(naku),
                KaniWordDispatchEnum::Kana(narimashita),
            ],
            score_base: None,
            score_mod: ScoreMod::Stack(vec![ScoreMod::Single(1), ScoreMod::Single(5)]),
        };
        let word = KaniWordDispatchEnum::Compound(compound);

        let (score, info) = calc_score(&ctx, &word, false, None, None, &[])
            
            .unwrap();
        assert_eq!(score, 0);
        let info = info.expect(
            "compound + skip-word inner must synthesize partial info \
             carrying the outer word_conj_data",
        );
        assert_eq!(
            info.conj.len(),
            1,
            "info.conj: expected single CONJ-DATA from last word なりました; got {:?}",
            info.conj,
        );
        let cd = &info.conj[0];
        assert_eq!(cd.seq, Some(10374833));
        assert_eq!(cd.from, Some(1375610));
        assert_eq!(cd.via, None);
        let prop = cd.prop.as_ref().expect("conj-prop present");
        assert_eq!(prop.id, 386572);
        assert_eq!(prop.conj_id, 378046);
        assert_eq!(prop.conj_type, 2);
        assert_eq!(prop.pos, "v5r");
        // Postgres false (`f`) decodes to Some(false), not None.
        assert_eq!(prop.neg, Some(false));
        assert_eq!(prop.fml, Some(true));
        assert_eq!(
            cd.src_map,
            vec![("なりました".to_string(), "なる".to_string())],
        );
        // Same zero/empty defaults for the five unsynthesized fields.
        assert!(info.posi.is_empty());
        assert!(info.seq_set.is_empty());
        assert_eq!(info.common, None);
        assert_eq!(info.score_info.prop_score, 0);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, false, false, false));
    }
}
