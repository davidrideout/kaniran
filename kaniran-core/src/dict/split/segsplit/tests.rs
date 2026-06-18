mod _star_segsplit_map_star_ {
    use crate::dict::split::segsplit::*;

    #[test]
    fn registered_count_matches_upstream_segsplit_map() {
        assert_eq!(REGISTERED_COUNT, 18);
    }
}

mod _star_hint_simplify_map_star_ {
    use crate::dict::split::segsplit::*;

    #[test]
    fn matches_introspected_value() {
        let map = hint_simplify_map();
        assert_eq!(map.len(), 6);
        assert_eq!(map[0], ("\u{200b}".to_string(), " "));
        assert_eq!(map[1], ("\u{200c}は".to_string(), "わ"));
        assert_eq!(map[2], ("\u{200c}ハ".to_string(), "ワ"));
        assert_eq!(map[3], ("\u{200c}へ".to_string(), "え"));
        assert_eq!(map[4], ("\u{200c}ヘ".to_string(), "エ"));
        assert_eq!(map[5], ("\u{200c}".to_string(), ""));
    }
}

mod get_segsplit {
    use crate::dict::readings::{find_word, FindWordRows};
    use crate::dict::scoring::score::gen_score;
    use crate::dict::split::segsplit::*;

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// Helper: find the reading for `text` at `seq`, gen-score it, return the segment.
    fn segment_for_seq(ctx: &KaniranContext, text: &str, seq: i32) -> Segment {
        let rows = find_word(ctx, text, false).unwrap().into_owned();
        let word = match rows {
            FindWordRows::Kana(v) => v
                .into_iter()
                .find(|k| k.seq == seq)
                .map(KaniWordDispatchEnum::Kana)
                .unwrap_or_else(|| panic!("no kana row for {} seq={}", text, seq)),
            FindWordRows::Kanji(v) => v
                .into_iter()
                .find(|k| k.seq == seq)
                .map(KaniWordDispatchEnum::Kanji)
                .unwrap_or_else(|| panic!("no kanji row for {} seq={}", text, seq)),
        };
        let mut seg = Segment {
            start: 0,
            end: text.chars().count(),
            word,
            score: None,
            info: None,
            top: None,
            text: Some(text.into()),
        };
        gen_score(ctx, &mut seg, false, &[]).unwrap();
        seg
    }

    fn assert_compound(seg: &Segment) -> &CompoundText {
        match &seg.word {
            KaniWordDispatchEnum::Compound(c) => c,
            other => panic!("expected compound-text segment, got {:?}", other),
        }
    }

    fn word_seq(w: &KaniWordDispatchEnum) -> i32 {
        match w {
            KaniWordDispatchEnum::Kanji(k) => k.seq,
            KaniWordDispatchEnum::Kana(k) => k.seq,
            _ => panic!("expected simple-text word in segsplit parts"),
        }
    }

    fn word_text(w: &KaniWordDispatchEnum) -> &str {
        match w {
            KaniWordDispatchEnum::Kanji(k) => &k.text,
            KaniWordDispatchEnum::Kana(k) => &k.text,
            _ => panic!("expected simple-text word in segsplit parts"),
        }
    }

    #[test]
    fn tokorode_root_keyword_marks_index_1() {
        let ctx = ctx_from_env();
        let mut seg = segment_for_seq(&ctx, "ところで", 1343110);
        // Set non-default start/end/top so the test catches any regression
        // that zeroes them.
        seg.start = 11;
        seg.end = 15;
        seg.top = None;
        let pre_score = seg.score.expect("gen-score sets score");
        let result = get_segsplit(&ctx, &seg).unwrap();
        let new_seg = result.expect("ところで matches split-tokorode (segsplit-map)");

        // start/end/top are carried over unchanged; only word/text/score/info
        // are rewritten.
        assert_eq!(new_seg.start, 11);
        assert_eq!(new_seg.end, 15);
        assert!(new_seg.top.is_none());

        let compound = assert_compound(&new_seg);
        assert_eq!(compound.text, "ところで");
        assert_eq!(compound.kana, "ところ で");
        assert_eq!(word_seq(&compound.primary), 1343100);
        assert_eq!(compound.words.len(), 2);
        assert_eq!(word_seq(&compound.words[0]), 1343100);
        assert_eq!(word_text(&compound.words[0]), "ところ");
        assert_eq!(word_seq(&compound.words[1]), 2028980);
        assert_eq!(word_text(&compound.words[1]), "で");
        // The index-1 word (で) is marked as a root.
        assert!(matches!(
            crate::dict::accessors::word_conjugations(&compound.words[1]),
            Some(WordConjugations::Root)
        ));
        assert!(crate::dict::accessors::word_conjugations(&compound.words[0]).is_none());
        assert!(matches!(compound.score_mod, ScoreMod::Single(-10)));
        assert!(compound.score_base.is_none());

        assert_eq!(new_seg.score, Some(pre_score - 10));
        assert_eq!(new_seg.text.as_deref(), Some("ところで"));
        let info = new_seg.info.as_ref().expect("segsplit info plist non-nil");
        assert_eq!(info.posi, vec!["adv", "n", "suf"]);
        assert_eq!(info.seq_set, vec![1343100]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 16);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, false));
    }

    // A three-part split (ところ / で / は) with default attributes.
    #[test]
    fn tokorodewa_three_part_split_default_attrs() {
        let ctx = ctx_from_env();
        let mut seg = segment_for_seq(&ctx, "ところでは", 1897510);
        seg.start = 3;
        seg.end = 8;
        seg.top = None;
        let pre_score = seg.score.expect("gen-score sets score");
        let new_seg = get_segsplit(&ctx, &seg)
            
            .unwrap()
            .expect("segsplit hit");

        assert_eq!(new_seg.start, 3);
        assert_eq!(new_seg.end, 8);
        assert!(new_seg.top.is_none());

        let compound = assert_compound(&new_seg);
        assert_eq!(compound.text, "ところでは");
        assert_eq!(compound.kana, "ところ で ‌は");
        assert_eq!(compound.words.len(), 3);
        assert_eq!(word_seq(&compound.words[0]), 1343100);
        assert_eq!(word_seq(&compound.words[1]), 2028980);
        assert_eq!(word_seq(&compound.words[2]), 2028920);
        // No root keyword, so no word is marked as a root.
        for w in &compound.words {
            assert!(crate::dict::accessors::word_conjugations(w).is_none());
        }
        assert!(matches!(compound.score_mod, ScoreMod::Single(-10)));
        assert!(compound.score_base.is_none());
        assert_eq!(new_seg.score, Some(pre_score - 10));
        assert_eq!(new_seg.text.as_deref(), Some("ところでは"));
        assert_eq!(word_seq(&compound.primary), 1343100);

        let info = new_seg.info.as_ref().expect("info set");
        assert_eq!(info.posi, vec!["adv", "n", "suf"]);
        assert_eq!(info.seq_set, vec![1343100]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 16);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, false));
    }

    // お店 splits into お / 店, with the second part as primary and an
    // empty connector joining the kana directly (おみせ).
    #[test]
    fn omise_primary_and_connector_keywords() {
        let ctx = ctx_from_env();
        let mut seg = segment_for_seq(&ctx, "お店", 2409240);
        seg.start = 4;
        seg.end = 6;
        seg.top = None;
        let pre_score = seg.score.expect("gen-score sets score");
        let new_seg = get_segsplit(&ctx, &seg)
            
            .unwrap()
            .expect("segsplit hit");

        assert_eq!(new_seg.start, 4);
        assert_eq!(new_seg.end, 6);
        assert!(new_seg.top.is_none());

        let compound = assert_compound(&new_seg);
        assert_eq!(compound.text, "お店");
        assert_eq!(compound.kana, "おみせ");
        assert_eq!(word_seq(&compound.primary), 1582120);
        assert_eq!(compound.words.len(), 2);
        assert_eq!(word_seq(&compound.words[0]), 2826528);
        assert_eq!(word_seq(&compound.words[1]), 1582120);
        for w in &compound.words {
            assert!(crate::dict::accessors::word_conjugations(w).is_none());
        }
        assert!(matches!(compound.score_mod, ScoreMod::Single(20)));
        assert!(compound.score_base.is_none());
        assert_eq!(new_seg.score, Some(pre_score + 20));
        assert_eq!(new_seg.text.as_deref(), Some("お店"));

        let info = new_seg.info.as_ref().expect("info set");
        assert_eq!(info.posi, vec!["n"]);
        assert_eq!(info.seq_set, vec![1582120]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 16);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (true, true, true, false));
    }

    // だから splits into だ / から with default attributes, the default
    // connector joining the kana with a space (だ から).
    #[test]
    fn dakara_basic_default_attrs() {
        let ctx = ctx_from_env();
        let mut seg = segment_for_seq(&ctx, "だから", 1007310);
        seg.start = 2;
        seg.end = 5;
        seg.top = None;
        let pre_score = seg.score.expect("gen-score sets score");
        let new_seg = get_segsplit(&ctx, &seg)
            
            .unwrap()
            .expect("segsplit hit");

        assert_eq!(new_seg.start, 2);
        assert_eq!(new_seg.end, 5);
        assert!(new_seg.top.is_none());

        let compound = assert_compound(&new_seg);
        assert_eq!(compound.text, "だから");
        assert_eq!(compound.kana, "だ から");
        assert_eq!(word_seq(&compound.primary), 2089020);
        assert_eq!(word_seq(&compound.words[0]), 2089020);
        assert_eq!(word_seq(&compound.words[1]), 1002980);
        assert!(matches!(compound.score_mod, ScoreMod::Single(-5)));
        assert!(compound.score_base.is_none());
        assert_eq!(new_seg.score, Some(pre_score - 5));
        assert_eq!(new_seg.text.as_deref(), Some("だから"));
        // No root keyword, so neither word is marked.
        for w in &compound.words {
            assert!(crate::dict::accessors::word_conjugations(w).is_none());
        }

        let info = new_seg.info.as_ref().expect("info set");
        assert_eq!(info.posi, vec!["aux-v", "cop", "cop-da"]);
        assert_eq!(info.seq_set, vec![2089020]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 16);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, false));
    }

    // から元気 splits into から / 元気 with the second part (the kanji) as
    // primary.
    #[test]
    fn karagenki_kanji_input_primary_1() {
        let ctx = ctx_from_env();
        let mut seg = segment_for_seq(&ctx, "から元気", 1675330);
        seg.start = 6;
        seg.end = 10;
        seg.top = None;
        let pre_score = seg.score.expect("gen-score sets score");
        let new_seg = get_segsplit(&ctx, &seg)
            
            .unwrap()
            .expect("segsplit hit");

        assert_eq!(new_seg.start, 6);
        assert_eq!(new_seg.end, 10);
        assert!(new_seg.top.is_none());

        let compound = assert_compound(&new_seg);
        assert_eq!(compound.text, "から元気");
        assert_eq!(compound.kana, "から げんき");
        assert_eq!(word_seq(&compound.primary), 1260720);
        assert_eq!(compound.words.len(), 2);
        assert_eq!(word_seq(&compound.words[0]), 1002980);
        assert_eq!(word_seq(&compound.words[1]), 1260720);
        for w in &compound.words {
            assert!(crate::dict::accessors::word_conjugations(w).is_none());
        }
        assert!(matches!(compound.score_mod, ScoreMod::Single(10)));
        assert!(compound.score_base.is_none());
        assert_eq!(new_seg.score, Some(pre_score + 10));
        assert_eq!(new_seg.text.as_deref(), Some("から元気"));

        let info = new_seg.info.as_ref().expect("info set");
        assert_eq!(info.posi, vec!["adj-na", "n"]);
        assert_eq!(info.seq_set, vec![1260720]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(6));
        assert_eq!(info.score_info.prop_score, 20);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (true, true, true, false));
    }

    // A word with no registered split returns nothing.
    #[test]
    fn neko_no_segsplit() {
        let ctx = ctx_from_env();
        let seg = segment_for_seq(&ctx, "猫", 1467640);
        let result = get_segsplit(&ctx, &seg).unwrap();
        assert!(result.is_none(), "猫 (1467640) is not in *segsplit-map*");
    }

    // はぐったり is a conjugated form, so the split is found via the word's
    // base form rather than a direct seq match — the only test here that
    // exercises that fallback path.
    #[test]
    fn hagguttari_conj_of_fallback_dispatch() {
        let ctx = ctx_from_env();
        let mut seg = segment_for_seq(&ctx, "はぐったり", 10494835);
        seg.start = 9;
        seg.end = 14;
        seg.top = None;
        let pre_score = seg.score.expect("gen-score sets score");
        let new_seg = get_segsplit(&ctx, &seg)
            
            .unwrap()
            .expect("segsplit hit via conj-of");

        assert_eq!(new_seg.start, 9);
        assert_eq!(new_seg.end, 14);
        assert!(new_seg.top.is_none());

        let compound = assert_compound(&new_seg);
        assert_eq!(compound.text, "はぐったり");
        assert_eq!(compound.kana, "‌は ぐったり");
        assert_eq!(word_seq(&compound.primary), 2028920);
        assert_eq!(compound.words.len(), 2);
        assert_eq!(word_seq(&compound.words[0]), 2028920);
        assert_eq!(word_text(&compound.words[0]), "は");
        assert_eq!(word_seq(&compound.words[1]), 1004070);
        assert_eq!(word_text(&compound.words[1]), "ぐったり");
        for w in &compound.words {
            assert!(crate::dict::accessors::word_conjugations(w).is_none());
        }
        assert!(matches!(compound.score_mod, ScoreMod::Single(5)));
        assert!(compound.score_base.is_none());
        assert_eq!(new_seg.score, Some(pre_score + 5));
        assert_eq!(new_seg.text.as_deref(), Some("はぐったり"));

        let info = new_seg.info.as_ref().expect("info set");
        assert_eq!(info.posi, vec!["prt"]);
        assert_eq!(info.seq_set, vec![2028920]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 11);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, false));
    }

    // A compound-text input is not a simple word, so it returns nothing.
    #[test]
    fn compound_text_input_returns_none() {
        use crate::dict::dao::KanaText;
        use crate::dict::dao::SimpleText;
        let ctx = ctx_from_env();
        let kana = KanaText {
            id: 0,
            seq: 1,
            text: "ところ".into(),
            ord: 0,
            common: None,
            common_tags: String::new().into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        };
        let inner = KaniWordDispatchEnum::Kana(kana);
        let compound = CompoundText {
            text: "ところで".into(),
            kana: "ところで".into(),
            primary: Box::new(inner.clone()),
            words: vec![inner.clone(), inner],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let seg = Segment {
            start: 0,
            end: 4,
            word: KaniWordDispatchEnum::Compound(compound),
            score: Some(100),
            info: Some(KaniSegmentInfo {
                posi: Vec::new(),
                seq_set: vec![1343110],
                conj: Vec::new(),
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: Vec::new(),
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl: (false, false, false, false),
            }),
            top: None,
            text: Some("ところで".into()),
        };
        let result = get_segsplit(&ctx, &seg).unwrap();
        assert!(result.is_none(), "compound-text input fails typep check");
    }

    // The result keeps the source segment's start/end/top; only
    // word/text/score/info are rewritten.
    #[test]
    fn copy_segment_preserves_start_end_top() {
        let ctx = ctx_from_env();
        let mut seg = segment_for_seq(&ctx, "だから", 1007310);
        // Re-anchor at non-default start/end so the test catches any drift.
        seg.start = 7;
        seg.end = 10;
        seg.top = None;
        let new_seg = get_segsplit(&ctx, &seg).unwrap().unwrap();
        assert_eq!(new_seg.start, 7);
        assert_eq!(new_seg.end, 10);
        assert!(new_seg.top.is_none());
    }
}
