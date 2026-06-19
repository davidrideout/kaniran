mod find_word_with_conj_prop {
    use crate::dict::grammar::lookup::*;

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL: `(find-word-with-conj-prop "食べて" (lambda (cd) t))` →
    /// 1 word, allow_root=nil. Filter accepts every cdata.
    #[test]
    fn t1_all_pass_no_allow_root() {
        let ctx = ctx();
        let r = find_word_with_conj_prop(&ctx, "食べて", |_| true, false)
            
            .unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        crate::test_support::check_base_seqs(k.seq, &[1358280]);
        assert!(matches!(
            &k.state.conjugations,
            Some(WordConjugations::Ids(ids)) if ids.len() == 1
        ));
    }

    /// REPL: `(find-word-with-conj-prop "食べて" (lambda (cd) t)
    /// :allow-root t)` → 1 word. allow_root doesn't change the
    /// outcome when conj-data is non-empty.
    #[test]
    fn t2_all_pass_with_allow_root() {
        let ctx = ctx();
        let r = find_word_with_conj_prop(&ctx, "食べて", |_| true, true)
            
            .unwrap();
        assert_eq!(r.len(), 1);
    }

    /// REPL: `(find-word-with-conj-prop "食べる" (lambda (cd) t)
    /// :allow-root t)` → 1 word, wc=NIL. 食べる is a root: empty
    /// conj-data + allow_root → collect with conj_ids=nil.
    #[test]
    fn t3_root_passthrough_with_allow_root() {
        let ctx = ctx();
        let r = find_word_with_conj_prop(&ctx, "食べる", |_| true, true)
            
            .unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 1358280);
        // Empty mapcar over nil → setter called with nil → None.
        assert_eq!(k.state.conjugations, None);
    }

    /// REPL: `(find-word-with-conj-prop "食べる" (lambda (cd) t))` →
    /// NIL. Without allow_root, root word is filtered out.
    #[test]
    fn t4_root_dropped_without_allow_root() {
        let ctx = ctx();
        let r = find_word_with_conj_prop(&ctx, "食べる", |_| true, false)
            
            .unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-conj-prop "食べて" (lambda (cd) nil))` →
    /// NIL. Filter rejects everything; without allow_root, no
    /// collection.
    #[test]
    fn t5_reject_all() {
        let ctx = ctx();
        let r = find_word_with_conj_prop(&ctx, "食べて", |_| false, false)
            
            .unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-conj-prop "食べて" (lambda (cd) nil)
    /// :allow-root t)` → NIL. Filter rejects all + word has conj-data
    /// → not the empty-conj-data-allow-root branch.
    #[test]
    fn t6_reject_all_with_allow_root_keeps_filtering() {
        let ctx = ctx();
        let r = find_word_with_conj_prop(&ctx, "食べて", |_| false, true)
            
            .unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-conj-prop "食べる" (lambda (cd) nil)
    /// :allow-root t)` → 1 word. Reject-all but conj-data empty +
    /// allow_root fires.
    #[test]
    fn t7_reject_all_empty_conj_data_allow_root() {
        let ctx = ctx();
        let r = find_word_with_conj_prop(&ctx, "食べる", |_| false, true)
            
            .unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 1358280);
        assert_eq!(k.state.conjugations, None);
    }

    /// REPL: `(find-word-with-conj-prop "食べなくて"
    ///   (lambda (cd) (conj-neg (conj-data-prop cd))))` → 1 word
    /// (neg = T, BOOLEAN).
    /// Filter mirrors Lisp truthiness for `(conj-neg ...)`: in CL only
    /// `nil` is falsy, so both `t` and `:NULL` count. Translated to
    /// `Option<bool>` that means `p.neg != Some(false)` (None / :NULL
    /// → truthy per memory `feedback_null_nil_truthy`).
    #[test]
    fn t8_neg_filter_matches() {
        let ctx = ctx();
        let r = find_word_with_conj_prop(
            &ctx,
            "食べなくて",
            |cd| cd.prop.as_ref().is_some_and(|p| p.neg != Some(false)),
            false,
        )
        
        .unwrap();
        assert_eq!(r.len(), 1);
    }
}

mod find_word_with_conj_type {
    use crate::dict::grammar::lookup::*;

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL: `(find-word-with-conj-type "食べて" 3)` → 1 word
    /// text=食べて seq=10092233 wc=(92707).
    #[test]
    fn t1_conj_type_3_matches() {
        let ctx = ctx();
        let r = find_word_with_conj_type(&ctx, "食べて", &[3])
            
            .unwrap();
        assert_eq!(r.len(), 1);
        let crate::dict::kani_word::KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        crate::test_support::check_base_seqs(k.seq, &[1358280]);
        assert!(matches!(
            &k.state.conjugations,
            Some(crate::dict::dao::WordConjugations::Ids(ids)) if ids.len() == 1
        ));
    }

    /// REPL: `(find-word-with-conj-type "食べ" 13)` → 1 word
    /// text=食べ seq=10092273 wc=(92747). Type 13 is ren'youkei stem.
    #[test]
    fn t2_conj_type_13() {
        let ctx = ctx();
        let r = find_word_with_conj_type(&ctx, "食べ", &[13]).unwrap();
        assert_eq!(r.len(), 1);
        let crate::dict::kani_word::KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        crate::test_support::check_base_seqs(k.seq, &[1358280]);
    }

    /// REPL: `(find-word-with-conj-type "食べる" 3)` → NIL. 食べる is a
    /// root, not a -te form.
    #[test]
    fn t3_no_match_for_root() {
        let ctx = ctx();
        let r = find_word_with_conj_type(&ctx, "食べる", &[3])
            
            .unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-conj-type "食べ" 3 13)` → 1 word (type
    /// 13 hits; type 3 doesn't). Exercises the multi-conj-type set
    /// `(member x '(3 13))` arm.
    #[test]
    fn t4_multi_conj_types() {
        let ctx = ctx();
        let r = find_word_with_conj_type(&ctx, "食べ", &[3, 13])
            
            .unwrap();
        assert_eq!(r.len(), 1);
    }

    /// REPL: `(find-word-with-conj-type "ジャバスクリプト" 3)` → NIL.
    /// Word with no conjugations.
    #[test]
    fn t5_no_conj_data() {
        let ctx = ctx();
        let r = find_word_with_conj_type(&ctx, "ジャバスクリプト", &[3])
            
            .unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-conj-type "abc" 3)` → NIL. No dictionary
    /// entry at all.
    #[test]
    fn t6_no_entry() {
        let ctx = ctx();
        let r = find_word_with_conj_type(&ctx, "abc", &[3]).unwrap();
        assert!(r.is_empty());
    }

    /// Empty `conj_types` mirrors `(find-word-with-conj-type "食べて")`
    /// — the closure `(member x nil)` is nil for every cdata; filter
    /// drops everything; allow_root=false; nothing collected.
    #[test]
    fn t7_empty_conj_types() {
        let ctx = ctx();
        let r = find_word_with_conj_type(&ctx, "食べて", &[]).unwrap();
        assert!(r.is_empty());
    }
}

mod pair_words_by_conj {
    use crate::dict::grammar::lookup::*;
    use crate::dict::dao::KanaText;
    use crate::dict::dao::SimpleText;

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    fn kana(seq: i32, text: &str, conj_ids: Vec<i32>) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq,
            text: text.to_string().into(),
            ord: 0,
            common: None,
            common_tags: String::new().into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText {
                conjugations: Some(WordConjugations::Ids(conj_ids)),
                hintedp: false,
            },
        })
    }

    fn seq_of(w: &KaniWordDispatchEnum) -> i32 {
        match w {
            KaniWordDispatchEnum::Kana(k) => k.seq,
            KaniWordDispatchEnum::Kanji(k) => k.seq,
            _ => panic!("test fixture only uses simple-text"),
        }
    }

    /// Sort buckets by `(idx0_seq, idx1_seq, …)` so we can deterministically
    /// compare against the REPL-captured pairing despite HashMap order.
    fn canonical(buckets: Vec<Vec<Option<KaniWordDispatchEnum>>>) -> Vec<Vec<Option<i32>>> {
        let mut rows: Vec<Vec<Option<i32>>> = buckets
            .into_iter()
            .map(|b| b.into_iter().map(|c| c.map(|w| seq_of(&w))).collect())
            .collect();
        rows.sort();
        rows
    }

    /// REPL: `(pair-words-by-conj)` → `NIL`. Length 0.
    #[test]
    fn no_args_returns_empty() {
        let ctx = ctx_from_env();
        let result = pair_words_by_conj(&ctx, &[]).unwrap();
        assert!(result.is_empty());
    }

    /// REPL: `(pair-words-by-conj nil nil nil)` → `NIL`. Length 0
    /// (no words anywhere → no bucket ever created).
    #[test]
    fn all_empty_groups_returns_empty() {
        let ctx = ctx_from_env();
        let result = pair_words_by_conj(&ctx, &[vec![], vec![], vec![]])
            
            .unwrap();
        assert!(result.is_empty());
    }

    /// REPL: `(pair-words-by-conj (find-word-with-conj-type "あった" 2))`
    /// → 3 buckets, each of length 1 holding one of the three あった
    /// readings. Each word has a distinct (seq-from, via) signature so
    /// no merging happens within the single group.
    #[test]
    fn single_group_three_distinct_keys() {
        let ctx = ctx_from_env();
        // The three あった entries conjugate from distinct bases (会う
        // 1198180/via-NULL, 合う 1284430/via-NULL, 有る 1296400/via-NULL), so
        // each gets its own (seq-from, via) key and its own bucket.
        let atta: Vec<(i32, Vec<i32>)> = [1198180, 1284430, 1296400]
            .iter()
            .map(|&base| crate::test_support::conj_word_parts("あった", base))
            .collect();
        let mut expected: Vec<Vec<Option<i32>>> =
            atta.iter().map(|(seq, _)| vec![Some(*seq)]).collect();
        expected.sort();
        let g1: Vec<KaniWordDispatchEnum> = atta
            .into_iter()
            .map(|(seq, ids)| kana(seq, "あった", ids))
            .collect();
        let result = pair_words_by_conj(&ctx, std::slice::from_ref(&g1)).unwrap();
        assert_eq!(canonical(result), expected);
    }

    /// REPL: `(pair-words-by-conj
    ///        (find-word-with-conj-type "あった" 2)
    ///        (find-word-with-conj-type "あったら" 11))`
    /// → 3 buckets pairing each あった with the matching あったら whose
    /// conj chain shares the same (seq-from, via).
    #[test]
    fn rashii_callsite_three_pairs() {
        let ctx = ctx_from_env();
        // あった and あったら each conjugate from the same three bases
        // (会う 1198180, 合う 1284430, 有る 1296400); the per-build synthetic
        // seqs and conj-row ids are resolved from those stable anchors.
        let bases = [1198180, 1284430, 1296400];
        let atta: Vec<(i32, Vec<i32>)> = bases
            .iter()
            .map(|&base| crate::test_support::conj_word_parts("あった", base))
            .collect();
        let attara: Vec<(i32, Vec<i32>)> = bases
            .iter()
            .map(|&base| crate::test_support::conj_word_parts("あったら", base))
            .collect();
        // Each base merges its あった with its あったら into one bucket.
        let mut expected: Vec<Vec<Option<i32>>> = atta
            .iter()
            .zip(&attara)
            .map(|((atta_seq, _), (attara_seq, _))| vec![Some(*atta_seq), Some(*attara_seq)])
            .collect();
        expected.sort();
        let g1: Vec<KaniWordDispatchEnum> = atta
            .into_iter()
            .map(|(seq, ids)| kana(seq, "あった", ids))
            .collect();
        let g2: Vec<KaniWordDispatchEnum> = attara
            .into_iter()
            .map(|(seq, ids)| kana(seq, "あったら", ids))
            .collect();
        let result = pair_words_by_conj(&ctx, &[g1, g2]).unwrap();
        assert_eq!(canonical(result), expected);
    }

    /// REPL: same callsite as `rashii_callsite_three_pairs`, but g2
    /// is empty → 3 buckets each holding `[Some(あった), None]`.
    #[test]
    fn second_group_empty_yields_none_slot() {
        let ctx = ctx_from_env();
        let (atta_seq, atta_ids) = crate::test_support::conj_word_parts("あった", 1198180);
        let g1 = vec![kana(atta_seq, "あった", atta_ids)];
        let g2: Vec<KaniWordDispatchEnum> = vec![];
        let result = pair_words_by_conj(&ctx, &[g1, g2]).unwrap();
        let expected: Vec<Vec<Option<i32>>> = vec![vec![Some(atta_seq), None]];
        assert_eq!(canonical(result), expected);
    }

    /// REPL: `(pair-words-by-conj nil (list w) nil)` where w has 1
    /// conjugation → 1 bucket of `[None, Some(w), None]`.
    #[test]
    fn middle_group_only_word_padding_on_both_sides() {
        let ctx = ctx_from_env();
        let (atta_seq, atta_ids) = crate::test_support::conj_word_parts("あった", 1198180);
        let g2 = vec![kana(atta_seq, "あった", atta_ids)];
        let result = pair_words_by_conj(&ctx, &[vec![], g2, vec![]]).unwrap();
        let expected: Vec<Vec<Option<i32>>> = vec![vec![None, Some(atta_seq), None]];
        assert_eq!(canonical(result), expected);
    }

    /// REPL: 立てた (conjs=[371171, 1210719]) and 立てたら
    /// (conjs=[371207, 1210739]) both reduce to the key
    /// [1551530, 1597040] → flatten [1551530,0,1597040,1551530]
    /// → single bucket containing the pair. Exercises the multi-conjugation
    /// sort path.
    #[test]
    fn multi_conjugation_words_share_a_bucket() {
        let ctx = ctx_from_env();
        // 立てた and 立てたら both carry two conjugation rows — direct from
        // 立てる (1551530) and via-1551530 from 立つ (1597040) — so their full
        // id sets reduce to the same key and share one bucket. Resolve the
        // per-build seqs/ids from the stable 立てる base.
        let (tateta_seq, tateta_ids) = crate::test_support::conj_word_parts("立てた", 1551530);
        let (tatetara_seq, tatetara_ids) =
            crate::test_support::conj_word_parts("立てたら", 1551530);
        let g1 = vec![kana(tateta_seq, "立てた", tateta_ids)];
        let g2 = vec![kana(tatetara_seq, "立てたら", tatetara_ids)];
        let result = pair_words_by_conj(&ctx, &[g1, g2]).unwrap();
        let expected: Vec<Vec<Option<i32>>> = vec![vec![Some(tateta_seq), Some(tatetara_seq)]];
        assert_eq!(canonical(result), expected);
    }
}

mod find_word_with_pos {
    use crate::dict::grammar::lookup::*;

    /// Kanji input → `kanji_text` dispatch with a single matching row.
    /// REPL: `(find-word-with-pos "区別" "vs")` → 1 KANJI-TEXT row
    /// id=13731, seq=1244250, common=10, best_kana=くべつ.
    #[test]
    fn kanji_single_match() {
        let ctx = crate::test_support::shared_ctx();
        let rows = find_word_with_pos(&ctx, "区別", &["vs"]).unwrap();
        let kanji = match rows {
            WordWithPosRows::Kanji(v) => v,
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 1);
        let row = &kanji[0];
        assert_eq!(row.seq, 1244250);
        assert_eq!(row.text, "区別");
        assert_eq!(row.ord, 0);
        assert_eq!(row.common, Some(10));
        assert_eq!(row.common_tags, "[ichi1][news1][nf10]");
        assert!(row.conjugate_p);
        assert!(!row.nokanji);
        assert_eq!(row.best_kana.as_deref(), Some("くべつ"));
    }

    /// Pure-katakana input → `test_word :kana` true → `kana_text`
    /// dispatch. REPL: `(find-word-with-pos "ジョギング" "vs")` →
    /// 1 KANA-TEXT row id=9654, seq=1066360, best_kanji = :NULL (the
    /// Lisp `:NULL` sentinel maps to Rust `None`).
    #[test]
    fn kana_single_match() {
        let ctx = crate::test_support::shared_ctx();
        let rows = find_word_with_pos(&ctx, "ジョギング", &["vs"])
            
            .unwrap();
        let kana = match rows {
            WordWithPosRows::Kana(v) => v,
            WordWithPosRows::Kanji(_) => panic!("expected Kana variant"),
        };
        assert_eq!(kana.len(), 1);
        let row = &kana[0];
        assert_eq!(row.seq, 1066360);
        assert_eq!(row.text, "ジョギング");
        assert_eq!(row.ord, 0);
        assert_eq!(row.common, Some(0));
        assert_eq!(row.common_tags, "[gai1][ichi1]");
        assert!(row.conjugate_p);
        assert!(!row.nokanji);
        assert_eq!(row.best_kanji, None);
    }

    /// Kanji word with no matching pos → empty `Kanji` result. REPL:
    /// `(find-word-with-pos "青空" "vs")` → 0 rows.
    #[test]
    fn kanji_no_match() {
        let ctx = crate::test_support::shared_ctx();
        let rows = find_word_with_pos(&ctx, "青空", &["vs"]).unwrap();
        match rows {
            WordWithPosRows::Kanji(v) => assert!(v.is_empty()),
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        }
    }

    /// `adj-i` pos tag. REPL: `(find-word-with-pos "赤い" "adj-i")` →
    /// 1 KANJI-TEXT row id=31416, seq=1383240.
    #[test]
    fn kanji_adj_i_match() {
        let ctx = crate::test_support::shared_ctx();
        let rows = find_word_with_pos(&ctx, "赤い", &["adj-i"]).unwrap();
        let kanji = match rows {
            WordWithPosRows::Kanji(v) => v,
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 1);
        assert_eq!(kanji[0].seq, 1383240);
        assert_eq!(kanji[0].common, Some(15));
        assert_eq!(kanji[0].best_kana.as_deref(), Some("あかい"));
    }

    /// `adj-na` pos tag. REPL: `(find-word-with-pos "好き" "adj-na")` →
    /// 1 KANJI-TEXT row id=17991, seq=1277450.
    #[test]
    fn kanji_adj_na_match() {
        let ctx = crate::test_support::shared_ctx();
        let rows = find_word_with_pos(&ctx, "好き", &["adj-na"]).unwrap();
        let kanji = match rows {
            WordWithPosRows::Kanji(v) => v,
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 1);
        assert_eq!(kanji[0].seq, 1277450);
        assert_eq!(kanji[0].common, Some(0));
        assert_eq!(kanji[0].best_kana.as_deref(), Some("すき"));
    }

    /// `pn` (pronoun) tag with a polysemous word → many rows. REPL:
    /// `(find-word-with-pos "私" "pn")` → 13 KANJI-TEXT rows. Pinned
    /// `(seq, id)` set captured from the REPL; row order is unspecified
    /// by the SQL (no ORDER BY upstream), so sort before comparison.
    #[test]
    fn kanji_pn_thirteen_rows() {
        let ctx = crate::test_support::shared_ctx();
        let rows = find_word_with_pos(&ctx, "私", &["pn"]).unwrap();
        let kanji = match rows {
            WordWithPosRows::Kanji(v) => v,
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 13);
        let mut got: Vec<i32> = kanji.iter().map(|r| r.seq).collect();
        got.sort();
        let expected: Vec<i32> = vec![
            1311110,
            1311125,
            1347580,
            2015370,
            2079310,
            2217330,
            2217340,
            2842390,
            2845454,
            2858221,
            2858384,
            2858397,
            2864027,
        ];
        assert_eq!(got, expected);
        for row in &kanji {
            assert_eq!(row.text, "私");
        }
    }

    /// ASCII input → not all kana → `kanji_text` dispatch, 0 rows.
    /// REPL: `(find-word-with-pos "nonsense" "vs")` → 0 rows.
    #[test]
    fn ascii_kanji_no_match() {
        let ctx = crate::test_support::shared_ctx();
        let rows = find_word_with_pos(&ctx, "nonsense", &["vs"]).unwrap();
        match rows {
            WordWithPosRows::Kanji(v) => assert!(v.is_empty()),
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        }
    }

    /// Multiple posi (exercise the `&rest` arity). REPL:
    /// `(find-word-with-pos "食べる" "v1" "vs")` → 1 KANJI-TEXT row
    /// id=28271, seq=1358280 (matches the `v1` pos).
    #[test]
    fn multi_pos_match() {
        let ctx = crate::test_support::shared_ctx();
        let rows = find_word_with_pos(&ctx, "食べる", &["v1", "vs"])
            
            .unwrap();
        let kanji = match rows {
            WordWithPosRows::Kanji(v) => v,
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 1);
        assert_eq!(kanji[0].seq, 1358280);
        assert_eq!(kanji[0].common, Some(25));
        assert_eq!(kanji[0].best_kana.as_deref(), Some("たべる"));
    }

    /// Kana word with multiple posi → `kana_text` dispatch, single row.
    /// REPL: `(find-word-with-pos "する" "vs-i" "vs-s")` →
    /// 1 KANA-TEXT row id=22268, seq=1157170.
    #[test]
    fn kana_multi_pos_match() {
        let ctx = crate::test_support::shared_ctx();
        let rows = find_word_with_pos(&ctx, "する", &["vs-i", "vs-s"])
            
            .unwrap();
        let kana = match rows {
            WordWithPosRows::Kana(v) => v,
            WordWithPosRows::Kanji(_) => panic!("expected Kana variant"),
        };
        assert_eq!(kana.len(), 1);
        assert_eq!(kana[0].seq, 1157170);
        assert_eq!(kana[0].common, Some(0));
        assert_eq!(kana[0].best_kanji.as_deref(), Some("為る"));
    }

    /// Polysemous kana word with three posi — exercises both the
    /// multi-posi `ANY` and the multi-row `SELECT DISTINCT` paths.
    /// REPL: `(find-word-with-pos "そう" "adv" "n" "aux-v")` → 26
    /// KANA-TEXT rows. Pinned `(seq, id)` set; sort before comparison.
    #[test]
    fn kana_three_pos_twentysix_rows() {
        let ctx = crate::test_support::shared_ctx();
        let rows = find_word_with_pos(&ctx, "そう", &["adv", "n", "aux-v"])
            
            .unwrap();
        let kana = match rows {
            WordWithPosRows::Kana(v) => v,
            WordWithPosRows::Kanji(_) => panic!("expected Kana variant"),
        };
        assert_eq!(kana.len(), 26);
        let mut got: Vec<i32> = kana.iter().map(|r| r.seq).collect();
        got.sort();
        let expected: Vec<i32> = vec![
            1241450,
            1398030,
            1398670,
            1399250,
            1399540,
            1399590,
            1399990,
            1400810,
            2027990,
            2033880,
            2137720,
            2249280,
            2253390,
            2406720,
            2414580,
            2414600,
            2639080,
            2681340,
            2843362,
            2843365,
            2843386,
            2843387,
            2843388,
            2843390,
            2843391,
            2844287,
        ];
        assert_eq!(got, expected);
    }
}

mod or_as_hiragana {
    use crate::dict::grammar::lookup::*;
    use crate::dict::grammar::lookup::{find_word_with_pos, WordWithPosRows};
    use crate::dict::kani_word::KaniSimpleTextDispatchEnum;

    // dict-grammar.lisp:506 (or-as-hiragana 'find-word-with-pos root …)
    fn make_pos_finder<'a>(ctx: &'a KaniranContext, posi: &'a [&'a str]) -> OrAsHiraganaFinder<'a> {
        Arc::new(move |word: String| {
            let rows = find_word_with_pos(ctx, &word, posi)?;
            Ok(match rows {
                WordWithPosRows::Kana(v) => FindWordRows::Kana(v),
                WordWithPosRows::Kanji(v) => FindWordRows::Kanji(v),
            })
        })
    }

    /// Path 1, kanji branch. REPL:
    /// `(or-as-hiragana 'find-word-with-pos "私" "pn")` → 13
    /// KANJI-TEXT rows (same as
    /// `(find-word-with-pos "私" "pn")` because "私" has no kana-only
    /// variant to displace it).
    #[test]
    fn kanji_direct_pn_thirteen_rows() {
        let ctx = crate::test_support::shared_ctx();
        let posi = ["pn"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "私", finder).unwrap();
        let direct = match result {
            Some(OrAsHiraganaRows::Direct(r)) => r,
            other => panic!("expected Direct, got {:?}", other),
        };
        let kanji = match direct {
            FindWordRows::Kanji(v) => v,
            FindWordRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 13);
        let mut got: Vec<i32> = kanji.iter().map(|r| r.seq).collect();
        got.sort();
        let expected: Vec<i32> = vec![
            1311110,
            1311125,
            1347580,
            2015370,
            2079310,
            2217330,
            2217340,
            2842390,
            2845454,
            2858221,
            2858384,
            2858397,
            2864027,
        ];
        assert_eq!(got, expected);
    }

    /// Path 1, kana branch (katakana that has a direct katakana
    /// kana-text row → short-circuit, no fallback). REPL:
    /// `(or-as-hiragana 'find-word-with-pos "ジョギング" "vs")` →
    /// 1 KANA-TEXT row id=9654 seq=1066360.
    #[test]
    fn katakana_direct_vs_match() {
        let ctx = crate::test_support::shared_ctx();
        let posi = ["vs"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "ジョギング", finder).unwrap();
        let direct = match result {
            Some(OrAsHiraganaRows::Direct(r)) => r,
            other => panic!("expected Direct, got {:?}", other),
        };
        let kana = match direct {
            FindWordRows::Kana(v) => v,
            FindWordRows::Kanji(_) => panic!("expected Kana variant"),
        };
        assert_eq!(kana.len(), 1);
        let row = &kana[0];
        assert_eq!(row.seq, 1066360);
        assert_eq!(row.text, "ジョギング");
        assert_eq!(row.ord, 0);
        assert_eq!(row.common, Some(0));
        assert_eq!(row.common_tags, "[gai1][ichi1]");
        assert!(row.conjugate_p);
        assert!(!row.nokanji);
        assert_eq!(row.best_kanji, None);
    }

    /// Path 1, hiragana branch (pure hiragana → `as-hiragana` is
    /// identity → fallback can't fire; only the direct call can
    /// match). REPL: `(or-as-hiragana 'find-word-with-pos "わたし"
    /// "pn")` → 1 KANA-TEXT row id=38072 seq=1311110.
    #[test]
    fn hiragana_direct_pn_match() {
        let ctx = crate::test_support::shared_ctx();
        let posi = ["pn"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "わたし", finder).unwrap();
        let direct = match result {
            Some(OrAsHiraganaRows::Direct(r)) => r,
            other => panic!("expected Direct, got {:?}", other),
        };
        let kana = match direct {
            FindWordRows::Kana(v) => v,
            FindWordRows::Kanji(_) => panic!("expected Kana variant"),
        };
        assert_eq!(kana.len(), 1);
        let row = &kana[0];
        assert_eq!(row.seq, 1311110);
        assert_eq!(row.text, "わたし");
        assert_eq!(row.ord, 0);
        assert_eq!(row.common, Some(1));
        assert_eq!(row.common_tags, "[ichi1][news1][nf01]");
        assert!(row.conjugate_p);
        assert!(!row.nokanji);
        assert_eq!(row.best_kanji.as_deref(), Some("私"));
    }

    /// Path 2a — katakana input with empty direct lookup but
    /// non-empty hiragana lookup. The fallback wraps each kana-text
    /// row in a proxy-text whose `text`/`kana` carry the original
    /// katakana surface form. REPL:
    /// `(or-as-hiragana 'find-word-with-pos "アナタ" "pn")` →
    /// 2 PROXY-TEXT rows; both wrap kana-text rows for "あなた"
    /// (ids 29081 / 55771, seqs 1223615 / 1483180).
    #[test]
    fn katakana_hiragana_fallback_two_proxies() {
        let ctx = crate::test_support::shared_ctx();
        let posi = ["pn"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "アナタ", finder).unwrap();
        let proxies = match result {
            Some(OrAsHiraganaRows::AsHiragana(p)) => p,
            other => panic!("expected AsHiragana, got {:?}", other),
        };
        assert_eq!(proxies.len(), 2);
        for proxy in &proxies {
            assert_eq!(proxy.text, "アナタ");
            assert_eq!(proxy.kana, "アナタ");
        }
        // Pin the stable (seq, text); the internal row `id` is an auto-
        // increment surrogate that renumbers per build.
        let mut sources: Vec<(i32, String)> = proxies
            .iter()
            .map(|p| match p.source.as_ref() {
                KaniSimpleTextDispatchEnum::Kana(row) => (row.seq, row.text.to_string()),
                KaniSimpleTextDispatchEnum::Kanji(row) => (row.seq, row.text.to_string()),
                KaniSimpleTextDispatchEnum::Proxy(_) => {
                    panic!("REPL pinned source to KANA-TEXT; got nested PROXY-TEXT")
                }
            })
            .collect();
        sources.sort();
        assert_eq!(
            sources,
            vec![
                (1223615, "あなた".to_string()),
                (1483180, "あなた".to_string()),
            ]
        );
    }

    /// Path None — both direct and hiragana lookup empty. REPL:
    /// `(or-as-hiragana 'find-word-with-pos "コノ" "pn")` → NIL
    /// (no kana-text or kanji-text rows for either katakana
    /// "コノ" or its hiragana form "この" with the "pn" pos tag).
    #[test]
    fn katakana_both_empty_yields_none() {
        let ctx = crate::test_support::shared_ctx();
        let posi = ["pn"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "コノ", finder).unwrap();
        assert!(result.is_none());
    }

    /// Path None — kanji input with no pos match; `as-hiragana`
    /// leaves kanji intact, so the fallback path also produces
    /// nothing (the str/as-hiragana equality short-circuit inside
    /// `find_word_as_hiragana` returns an empty Vec). REPL:
    /// `(or-as-hiragana 'find-word-with-pos "青空" "vs")` → NIL.
    #[test]
    fn kanji_no_match_yields_none() {
        let ctx = crate::test_support::shared_ctx();
        let posi = ["vs"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "青空", finder).unwrap();
        assert!(result.is_none());
    }
}

mod find_word_with_suffix {
    use crate::dict::grammar::lookup::*;

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL: `(find-word-with-suffix "我々ら" :ra)` → 1 compound
    /// text=我々ら kana=われわれら.
    #[test]
    fn t1_warera_ra_match() {
        let ctx = ctx();
        let r = find_word_with_suffix(&ctx, "我々ら", &["ra"])
            
            .unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected COMPOUND");
        };
        assert_eq!(c.text, "我々ら");
        assert_eq!(c.kana, "われわれら");
    }

    /// REPL: `(find-word-with-suffix "勉強する" :suru)` → 1 compound
    /// text=勉強する kana=べんきょう する.
    #[test]
    fn t2_benkyousuru_suru_match() {
        let ctx = ctx();
        let r = find_word_with_suffix(&ctx, "勉強する", &["suru"])
            
            .unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected COMPOUND");
        };
        assert_eq!(c.text, "勉強する");
        assert_eq!(c.kana, "べんきょう する");
    }

    /// REPL: `(find-word-with-suffix "勉強する" :ra)` → NIL (class
    /// mismatch).
    #[test]
    fn t3_wrong_class_drops() {
        let ctx = ctx();
        let r = find_word_with_suffix(&ctx, "勉強する", &["ra"])
            
            .unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-suffix "区別" :suru)` → NIL. Simple-text
    /// `seq` is an integer (not listp) — class lookup skipped.
    #[test]
    fn t4_simple_text_seq_not_listp() {
        let ctx = ctx();
        let r = find_word_with_suffix(&ctx, "区別", &["suru"])
            
            .unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-suffix "abc" :suru)` → NIL.
    #[test]
    fn t5_no_entries() {
        let ctx = ctx();
        let r = find_word_with_suffix(&ctx, "abc", &["suru"]).unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-suffix "勉強する")` → NIL. Empty
    /// suffix-classes — `(find x nil)` is always nil → no
    /// collection.
    #[test]
    fn t6_empty_classes() {
        let ctx = ctx();
        let r = find_word_with_suffix(&ctx, "勉強する", &[]).unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-suffix "勉強する" :ra :suru)` → 1
    /// compound (suru matches, ra doesn't). Multi-class set.
    #[test]
    fn t7_multi_class_set() {
        let ctx = ctx();
        let r = find_word_with_suffix(&ctx, "勉強する", &["ra", "suru"])
            
            .unwrap();
        assert_eq!(r.len(), 1);
    }
}
