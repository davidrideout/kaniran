mod exists_reading {
    use crate::dict::find_word_info::*;

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    #[test]
    fn reading_present_and_absent() {
        let ctx = ctx();
        assert_eq!(
            exists_reading(&ctx, 1376070, "せいふ").unwrap(),
            vec![1376070]
        );
        assert!(exists_reading(&ctx, 1376070, "ありえない")
            
            .unwrap()
            .is_empty());
        assert_eq!(
            exists_reading(&ctx, 1467640, "ねこ").unwrap(),
            vec![1467640]
        );
        // reading belongs to a different entry -> no row for this seq
        assert!(exists_reading(&ctx, 1467640, "せいふ")
            
            .unwrap()
            .is_empty());
    }
}

mod find_word_info {
    use crate::dict::find_word_info::*;

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    fn kana_of(wi: &WordInfo) -> &str {
        match &wi.kana {
            Some(WordInfoKana::Single(k)) => k,
            other => panic!("expected single kana, got {other:?}"),
        }
    }

    fn single_seq(wi: &WordInfo) -> i32 {
        match &wi.seq {
            Some(WordInfoSeq::Single(s)) => *s,
            other => panic!("expected single seq, got {other:?}"),
        }
    }

    /// One-result lookups, checking every populated field: kanji words,
    /// and katakana words that resolve to a kana row. For every simple
    /// word the true-text equals the surface text and the span runs from
    /// 0 to the text length.
    #[test]
    fn single_result_cases() {
        use crate::dict::word_info::WordInfoType;
        let ctx = ctx();
        // (text, kana, seq, score, is_kana_type)
        let cases: &[(&str, &str, i32, i32, bool)] = &[
            ("政府", "せいふ", 1376070, 325, false),
            ("経済", "けいざい", 1251320, 325, false),
            ("今日", "きょう", 1579110, 312, false),
            ("明日", "あした", 1584660, 273, false),
            ("ヨーロッパ", "ヨーロッパ", 1137570, 384, true),
            ("コンピューター", "コンピューター", 1053350, 440, true),
        ];
        for (text, kana, seq, score, is_kana) in cases {
            let result = find_word_info(&ctx, text, None, false).unwrap();
            assert_eq!(result.len(), 1, "text={text}");
            let wi = &result[0];
            assert_eq!(&wi.text, text, "text={text}");
            assert_eq!(kana_of(wi), *kana, "text={text}");
            assert_eq!(single_seq(wi), *seq, "text={text}");
            assert_eq!(wi.score, Some(*score), "text={text}");
            assert_eq!(
                wi.kind,
                if *is_kana {
                    WordInfoType::Kana
                } else {
                    WordInfoType::Kanji
                },
                "text={text}"
            );
            assert_eq!(wi.true_text.as_deref(), Some(*text), "text={text}");
            assert_eq!(wi.start, Some(0), "text={text}");
            assert_eq!(wi.end, Some(text.chars().count()), "text={text}");
            assert!(wi.counter.is_none(), "text={text}");
            assert!(wi.components.is_empty(), "text={text}");
        }
    }

    /// Multi-result lookups with distinct scores come back ordered
    /// strictly descending by score.
    #[test]
    fn multi_result_sorted_descending() {
        let ctx = ctx();
        // (text, [(kana, seq, score), …] in expected order)
        let cases: &[(&str, &[(&str, i32, i32)])] = &[
            ("何", &[("なに", 1577100, 24), ("なん", 2846738, 16)]),
            (
                "一人",
                &[("ひとり", 1576150, 312), ("ひとり", 2149890, 208)],
            ),
            (
                "二人",
                &[("ふたり", 1582670, 325), ("ふたり", 2149890, 208)],
            ),
        ];
        for (text, expected) in cases {
            let result = find_word_info(&ctx, text, None, false).unwrap();
            assert_eq!(result.len(), expected.len(), "text={text}");
            for (wi, (kana, seq, score)) in result.iter().zip(expected.iter()) {
                assert_eq!(&wi.text, text, "text={text}");
                assert_eq!(kana_of(wi), *kana, "text={text}");
                assert_eq!(single_seq(wi), *seq, "text={text}");
                assert_eq!(wi.score, Some(*score), "text={text}");
            }
        }
    }

    /// 三本 → 3 results: two tied at 208 then one at 143. The order
    /// between the two tied rows is unspecified, so the test asserts the
    /// descending score sequence and the seq set, not the tie order.
    #[test]
    fn score_tie_then_lower() {
        let ctx = ctx();
        let result = find_word_info(&ctx, "三本", None, false).unwrap();
        assert_eq!(result.len(), 3);
        let scores: Vec<i32> = result.iter().map(|wi| wi.score.unwrap()).collect();
        assert_eq!(scores, vec![208, 208, 143]);
        let mut seqs: Vec<i32> = result.iter().map(single_seq).collect();
        seqs.sort_unstable();
        assert_eq!(seqs, vec![1260670, 1301640, 1522150]);
        assert_eq!(single_seq(&result[2]), 1260670); // the 143 row sorts last
    }

    /// 5個 → 2 counter readings (ごこ 128 / ごか 40), each a counter word
    /// with a single source seq.
    #[test]
    fn counter_auto_results() {
        let ctx = ctx();
        let result = find_word_info(&ctx, "5個", None, false).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(
            (kana_of(&result[0]), single_seq(&result[0]), result[0].score),
            ("ごこ", 1264740, Some(128))
        );
        assert_eq!(
            (kana_of(&result[1]), single_seq(&result[1]), result[1].score),
            ("ごか", 2220320, Some(40))
        );
        // Counter words have no true-text; counter = (value-string,
        // ordinalp) and the span is 0..2.
        for wi in &result {
            assert_eq!(wi.counter, Some(("Value: 5".to_string(), false)));
            assert!(wi.true_text.is_none());
            assert_eq!(wi.start, Some(0));
            assert_eq!(wi.end, Some(2));
        }
    }

    /// Root-only lookup returns just the single root reading per word.
    #[test]
    fn root_only_cases() {
        let ctx = ctx();
        let cases: &[(&str, &str, i32, i32)] = &[
            ("経済", "けいざい", 1251320, 325),
            ("三本", "さんぼん", 1301640, 208),
            ("一人", "ひとり", 1576150, 312),
        ];
        for (text, kana, seq, score) in cases {
            let result = find_word_info(&ctx, text, None, true).unwrap();
            assert_eq!(result.len(), 1, "text={text}");
            assert_eq!(kana_of(&result[0]), *kana, "text={text}");
            assert_eq!(single_seq(&result[0]), *seq, "text={text}");
            assert_eq!(result[0].score, Some(*score), "text={text}");
        }
    }

    /// When the supplied reading equals the word's kana, the word is kept
    /// unchanged, including the compound 食べてる whose kana たべてる matches.
    #[test]
    fn reading_match_collects() {
        let ctx = ctx();
        let seifu = find_word_info(&ctx, "政府", Some("せいふ"), false)
            
            .unwrap();
        assert_eq!(seifu.len(), 1);
        assert_eq!(kana_of(&seifu[0]), "せいふ");
        assert_eq!(single_seq(&seifu[0]), 1376070);

        let taberu = find_word_info(&ctx, "食べてる", Some("たべてる"), false)
            
            .unwrap();
        assert_eq!(taberu.len(), 1);
        assert_eq!(kana_of(&taberu[0]), "たべてる");
        assert_eq!(
            taberu[0].seq,
            Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(10092233)),
                Some(WordInfoSeq::Single(1577980)),
            ]))
        );
    }

    /// When the reading differs from the kana but the seq still has that
    /// reading, the word's kana is relabeled to the reading and kept;
    /// rows whose seq lacks the reading are dropped.
    #[test]
    fn reading_relabel_and_drop() {
        let ctx = ctx();
        // (text, reading, expected (kana, seq, score))
        let cases: &[(&str, &str, &str, i32, i32)] = &[
            ("一人", "いちにん", "いちにん", 1576150, 312),
            ("今日", "こんにち", "こんにち", 1579110, 312),
            ("今日", "こんじつ", "こんじつ", 1579110, 312),
            ("二人", "ににん", "ににん", 1582670, 325),
            ("何", "なん", "なん", 2846738, 16),
        ];
        for (text, reading, kana, seq, score) in cases {
            let result = find_word_info(&ctx, text, Some(reading), false)
                
                .unwrap();
            assert_eq!(result.len(), 1, "text={text} reading={reading}");
            assert_eq!(kana_of(&result[0]), *kana, "text={text} reading={reading}");
            assert_eq!(
                single_seq(&result[0]),
                *seq,
                "text={text} reading={reading}"
            );
            assert_eq!(
                result[0].score,
                Some(*score),
                "text={text} reading={reading}"
            );
        }
    }

    /// When the reading matches no row for any seq, every word is dropped
    /// and the result is empty.
    #[test]
    fn reading_drops_all() {
        let ctx = ctx();
        assert!(find_word_info(&ctx, "政府", Some("ありえない"), false)
            
            .unwrap()
            .is_empty());
        assert!(find_word_info(&ctx, "何", Some("ぜんぜんちがう"), false)
            
            .unwrap()
            .is_empty());
    }

    /// Compounds carry a list of seqs and a per-part components list,
    /// each child marked primary only when it is the compound's primary.
    /// A compound has no true-text and its span covers the whole text.
    #[test]
    fn compound_results() {
        let ctx = ctx();
        // (text, kana, score, [(comp_text, comp_kana, comp_seq, primary)])
        let cases: &[(&str, &str, i32, &[(&str, &str, i32, bool)])] = &[
            (
                "食べてる",
                "たべてる",
                434,
                &[
                    ("食べて", "たべて", 10092233, true),
                    ("いる", "いる", 1577980, false),
                ],
            ),
            (
                "勉強する",
                "べんきょう する",
                736,
                &[
                    ("勉強", "べんきょう", 1512670, true),
                    ("する", "する", 1157170, false),
                ],
            ),
        ];
        for (text, kana, score, comps) in cases {
            let result = find_word_info(&ctx, text, None, false).unwrap();
            assert_eq!(result.len(), 1, "text={text}");
            let wi = &result[0];
            assert_eq!(&wi.text, text, "text={text}");
            assert_eq!(kana_of(wi), *kana, "text={text}");
            assert_eq!(wi.score, Some(*score), "text={text}");
            assert!(wi.true_text.is_none(), "text={text}");
            assert_eq!(wi.start, Some(0), "text={text}");
            assert_eq!(wi.end, Some(text.chars().count()), "text={text}");
            let expected_seq = WordInfoSeq::Multi(
                comps
                    .iter()
                    .map(|(_, _, s, _)| Some(WordInfoSeq::Single(*s)))
                    .collect(),
            );
            assert_eq!(wi.seq, Some(expected_seq), "text={text}");
            assert_eq!(wi.components.len(), comps.len(), "text={text}");
            for (comp, (comp_text, comp_kana, comp_seq, primary)) in
                wi.components.iter().zip(comps.iter())
            {
                assert_eq!(&comp.text, comp_text, "text={text}");
                assert_eq!(kana_of(comp), *comp_kana, "text={text}");
                assert_eq!(single_seq(comp), *comp_seq, "text={text}");
                assert_eq!(comp.primary, *primary, "text={text}");
            }
        }
    }

    /// A compound whose kana differs from the supplied reading triggers a
    /// reading lookup against its list of seqs, which the database rejects
    /// with SQLSTATE 42883.
    #[cfg(feature = "postgres")]
    #[test]
    fn compound_reading_mismatch_errors() {
        let ctx = ctx();
        let err = find_word_info(&ctx, "食べてる", Some("ちがうよみ"), false)
            
            .expect_err("compound list-seq exists-reading must raise a DB error");
        match err {
            crate::conn::KaniDbError::Database(db) => {
                // The Postgres backend boxes its `sqlx::Error` into the
                // backend-neutral `Database` variant; downcast back to reach
                // the SQLSTATE the probe was rejected with.
                let sqlx_err = db
                    .downcast_ref::<sqlx::Error>()
                    .expect("Database variant wraps a sqlx::Error");
                let code = sqlx_err.as_database_error().and_then(|db| db.code());
                assert_eq!(code.as_deref(), Some("42883"));
            }
            other => panic!("expected SQLSTATE 42883 database error, got {other:?}"),
        }
    }

    /// No dictionary hit → empty result.
    #[test]
    fn no_match_is_empty() {
        let ctx = ctx();
        assert!(find_word_info(&ctx, "qwxz", None, false)
            
            .unwrap()
            .is_empty());
    }
}

mod find_word_info_json {
    use crate::dict::find_word_info::*;

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    fn json(values: &[Value]) -> String {
        serde_json::to_string(values).unwrap()
    }

    /// JSON output for: a single-result noun; root-only (one object, no
    /// conjugation block); and root-only on a conjugated compound, which
    /// has no root entry and yields an empty list.
    #[test]
    fn find_word_info_json_cases() {
        let ctx = ctx_from_env();
        // (text, reading, root_only, expected list json)
        let cases: &[(&str, Option<&str>, bool, &str)] = &[
            (
                "経済",
                None,
                false,
                r#"[{"reading":"経済 【けいざい】","text":"経済","kana":"けいざい","score":325,"seq":1251320,"gloss":[{"pos":"[n]","gloss":"economy; economics"},{"pos":"[n]","gloss":"finance; (one's) finances; financial circumstances"},{"pos":"[n]","gloss":"being economical; economy; thrift"}],"conj":[]}]"#,
            ),
            (
                "経済",
                None,
                true,
                r#"[{"reading":"経済 【けいざい】","text":"経済","kana":"けいざい","score":325,"seq":1251320,"gloss":[{"pos":"[n]","gloss":"economy; economics"},{"pos":"[n]","gloss":"finance; (one's) finances; financial circumstances"},{"pos":"[n]","gloss":"being economical; economy; thrift"}]}]"#,
            ),
            ("行きたい", None, true, "[]"),
        ];
        for (text, reading, root_only, expected) in cases {
            let result = find_word_info_json(&ctx, text, *reading, *root_only)
                
                .unwrap();
            assert_eq!(json(&result), *expected, "text={text} root={root_only}");
        }
    }

    /// Supplying a reading keeps only the seq that has it and relabels the
    /// kana to that reading before serializing: 今日 with こんにち.
    #[test]
    fn reading_relabel() {
        let ctx = ctx_from_env();
        let result = find_word_info_json(&ctx, "今日", Some("こんにち"), false)
            
            .unwrap();
        assert_eq!(
            json(&result),
            r#"[{"reading":"今日 【こんにち】","text":"今日","kana":"こんにち","score":312,"seq":1579110,"gloss":[{"pos":"[n,adv]","gloss":"today; this day"},{"pos":"[n,adv]","gloss":"these days; recently; nowadays"}],"conj":[]}]"#
        );
    }
}

mod find_word_kana_pattern {
    use crate::dict::find_word_info::*;

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// Each pattern's rows come back ordered by `common`: positive ranks
    /// ascending, then 0, then nulls last. はし spans six homophones,
    /// あれ shows the 0 rank between positives and nulls, and ^xyzzlkj$
    /// matches nothing. Tied ranks keep their database scan order.
    #[test]
    fn common_sort_order() {
        let ctx = ctx_from_env();
        let cases: &[(&str, &str, Vec<Option<i32>>)] = &[
            (
                "^はし$",
                "はし",
                vec![Some(5), Some(5), Some(19), None, None, None],
            ),
            ("^あれ$", "あれ", vec![Some(21), Some(0), None, None]),
            ("^がっこう$", "がっこう", vec![Some(1), None]),
            ("^xyzzlkj$", "", vec![]),
        ];
        for (pattern, text, expected_commons) in cases {
            let rows = find_word_kana_pattern(&ctx, pattern).unwrap();
            assert!(
                rows.iter().all(|row| row.text == *text),
                "pattern={pattern:?}: every row text should be {text:?}"
            );
            let commons: Vec<Option<i32>> = rows.iter().map(|row| row.common).collect();
            assert_eq!(&commons, expected_commons, "pattern={pattern:?}");
        }
    }

    /// Single-row patterns return exactly that row.
    #[test]
    fn single_row_patterns() {
        let ctx = ctx_from_env();
        let cases: &[(&str, i32, i32, Option<i32>)] = &[
            // pattern, seq, id, common
            ("^ねこ$", 1467640, 54168, Some(7)),
            ("^きそうてんがい$", 1219430, 28651, Some(26)),
        ];
        for (pattern, seq, id, common) in cases {
            let rows = find_word_kana_pattern(&ctx, pattern).unwrap();
            assert_eq!(rows.len(), 1, "pattern={pattern:?}");
            assert_eq!(rows[0].seq, *seq, "pattern={pattern:?}");
            assert_eq!(rows[0].id, *id, "pattern={pattern:?}");
            assert_eq!(rows[0].common, *common, "pattern={pattern:?}");
        }
    }
}

mod find_kanji_for_pattern {
    use crate::dict::find_word_info::*;

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// For each kana pattern, the kanji forms and the deduplicated kana.
    /// つくえ has one kanji; がっこう orders kanji by `common` (学校 before
    /// 楽校); あれ skips a row that has no kanji and collapses four kana
    /// rows to one; ^xyzzlkj$ returns two empty lists.
    #[test]
    fn find_kanji_for_pattern_fixtures() {
        let ctx = ctx_from_env();
        let cases: &[(&str, Vec<&str>, Vec<&str>)] = &[
            ("^つくえ$", vec!["机"], vec!["つくえ"]),
            ("^がっこう$", vec!["学校", "楽校"], vec!["がっこう"]),
            ("^あれ$", vec!["荒れ", "彼", "有れ"], vec!["あれ"]),
            ("^xyzzlkj$", vec![], vec![]),
        ];
        for (pattern, expected_kanji, expected_kana) in cases {
            let (kanji, kana) = find_kanji_for_pattern(&ctx, pattern).unwrap();
            assert_eq!(&kanji, expected_kanji, "pattern={pattern:?} (kanji)");
            assert_eq!(&kana, expected_kana, "pattern={pattern:?} (kana)");
        }
    }

    #[test]
    fn dedup_keeps_first_occurrence() {
        // Dedup keeps the first occurrence of each value and preserves order.
        let input = vec![
            "橋".to_string(),
            "端".to_string(),
            "橋".to_string(),
            "箸".to_string(),
            "端".to_string(),
        ];
        assert_eq!(
            remove_duplicates_from_end(input),
            vec!["橋".to_string(), "端".to_string(), "箸".to_string()]
        );
    }
}

mod get_glosses {
    use crate::dict::find_word_info::*;
    // Needs a live database.

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// Glosses for several seqs: outer order is ascending seq; within a
    /// seq the glosses come back in reverse physical-row order.
    #[test]
    fn multi_seq_grouping_and_inner_reversal() {
        let ctx = ctx_from_env();
        let out = get_glosses(&ctx, &[1372640, 1577100]).unwrap();
        assert_eq!(
            out,
            vec![
                (
                    1372640,
                    vec!["execution".to_string(), "accomplishment".to_string()]
                ),
                (
                    1577100,
                    vec![
                        "oh (certainly not)".to_string(),
                        "why (it's nothing)".to_string(),
                        "oh, no (it's fine)".to_string(),
                        "come on!".to_string(),
                        "hey!".to_string(),
                        "huh?".to_string(),
                        "what?".to_string(),
                        "(not) in the slightest".to_string(),
                        "(not) at all".to_string(),
                        "dick".to_string(),
                        "(one's) thing".to_string(),
                        "penis".to_string(),
                        "what's-her-name".to_string(),
                        "what's-his-name".to_string(),
                        "whachamacallit".to_string(),
                        "whatsit".to_string(),
                        "that thing".to_string(),
                        "you-know-what".to_string(),
                        "what".to_string(),
                    ],
                ),
            ],
        );
    }

    /// No seqs in → empty out.
    #[test]
    fn empty_seqs_returns_empty() {
        let ctx = ctx_from_env();
        let out = get_glosses(&ctx, &[]).unwrap();
        assert!(out.is_empty());
    }

    /// An unknown seq returns a single JMdict header-row gloss.
    #[test]
    fn unknown_seq_returns_header_row() {
        let ctx = ctx_from_env();
        let out = get_glosses(&ctx, &[9999999]).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, 9999999);
        assert_eq!(out[0].1.len(), 1);
        assert!(out[0].1[0].starts_with("Japanese-Multilingual Dictionary Project"));
    }
}

mod get_candidates {
    use crate::dict::find_word_info::*;
    // Needs a live database.

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    #[test]
    fn kana_branch_no_root_kana_only_entry() {
        let ctx = ctx_from_env();
        let out = get_candidates(&ctx, "する", None).unwrap();
        assert!(out.is_empty(), "expected NIL, got {:?}", out);
    }

    #[test]
    fn kanji_branch_with_reading() {
        let ctx = ctx_from_env();
        let out = get_candidates(&ctx, "漢字", Some("かんじ")).unwrap();
        assert_eq!(out, vec![1213170]);
    }

    #[test]
    fn kana_branch_pure_katakana_hit() {
        let ctx = ctx_from_env();
        let out = get_candidates(&ctx, "テスト", None).unwrap();
        assert_eq!(out, vec![1079760]);
    }

    #[test]
    fn kana_branch_unknown_kana_returns_empty() {
        let ctx = ctx_from_env();
        let out = get_candidates(&ctx, "ジャバスクリプトーー", None)
            
            .unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn kanji_branch_bogus_reading_returns_empty() {
        let ctx = ctx_from_env();
        let out = get_candidates(&ctx, "漢字", Some("ZZZZZZZZ"))
            
            .unwrap();
        assert!(out.is_empty());
    }
}

mod match_glosses {
    use crate::dict::find_word_info::*;
    // Needs a live database.

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// When all requested words appear in a gloss, returns that seq and a
    /// found flag of true.
    #[test]
    fn words_match_returns_seq_true() {
        let ctx = ctx_from_env();
        let out = match_glosses(
            &ctx,
            "漢字",
            Some("かんじ"),
            &["Chinese", "character"],
            None,
            None,
        )
        
        .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), true)));
    }

    /// When no word matches, falls back to the first candidate seq with a
    /// found flag of false.
    #[test]
    fn no_word_match_fallback_to_first_candidate() {
        let ctx = ctx_from_env();
        let out = match_glosses(&ctx, "漢字", Some("かんじ"), &["zzzzz"], None, None)
            
            .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), false)));
    }

    /// When the text/reading yields no candidates, returns None.
    #[test]
    fn empty_candidates_returns_none() {
        let ctx = ctx_from_env();
        let out = match_glosses(&ctx, "漢字", Some("ZZZZZ"), &["x"], None, None)
            
            .unwrap();
        assert_eq!(out, None);
    }

    /// A matching update-gloss pattern returns the seq paired with the
    /// matched gloss text and a found flag of true.
    #[test]
    fn update_gloss_match_returns_seq_and_gloss() {
        let ctx = ctx_from_env();
        let rg = fancy_regex::Regex::new("(?i)^chinese character").unwrap();
        let out = match_glosses(&ctx, "漢字", Some("かんじ"), &["zzzz"], None, Some(&rg))
            
            .unwrap();
        assert_eq!(
            out,
            Some((
                MatchValue::SeqAndGloss(1213170, "Chinese character".to_string()),
                true
            )),
        );
    }

    /// When neither the update-gloss pattern nor the words match, falls
    /// back to the first candidate with a found flag of false.
    #[test]
    fn update_gloss_miss_and_words_miss_fallback() {
        let ctx = ctx_from_env();
        let rg = fancy_regex::Regex::new("XXXX").unwrap();
        let out = match_glosses(&ctx, "漢字", Some("かんじ"), &["zzzz"], None, Some(&rg))
            
            .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), false)));
    }

    /// A lowercase normalizer makes otherwise case-mismatched words match.
    #[test]
    fn normalize_string_downcase_enables_match() {
        let ctx = ctx_from_env();
        let downcase: &dyn Fn(&str) -> String = &|s: &str| s.to_lowercase();
        let out = match_glosses(
            &ctx,
            "漢字",
            Some("かんじ"),
            &["chinese", "character"],
            Some(downcase),
            None,
        )
        
        .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), true)));
    }

    /// Without a normalizer, the match is case-sensitive, so lowercase
    /// words don't match a capitalized gloss and it falls back.
    #[test]
    fn no_normalize_lowercase_falls_back() {
        let ctx = ctx_from_env();
        let out = match_glosses(
            &ctx,
            "漢字",
            Some("かんじ"),
            &["chinese", "character"],
            None,
            None,
        )
        
        .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), false)));
    }

    /// An empty word list matches the first gloss with a found flag of true.
    #[test]
    fn empty_words_matches_first_gloss() {
        let ctx = ctx_from_env();
        let out = match_glosses(&ctx, "漢字", Some("かんじ"), &[], None, None)
            
            .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), true)));
    }
}
