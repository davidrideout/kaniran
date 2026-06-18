mod suffix_tai {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:tai` suffix-cache `kf`, REPL pinned: `(get-kana-form 2017560
    /// "たい")` → id=109172, seq=2017560, text="たい", common=0,
    /// common_tags="[spec1]", conjugate_p=T, nokanji=nil,
    /// best_kanji=:NULL, hintedp=nil.
    fn kf_tai() -> KanaText {
        KanaText {
            id: 109172,
            seq: 2017560,
            text: "たい".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL TAI1: `(suffix-tai "食べ" "たい" kf-tai)` → 1 COMPOUND
    /// text="食べたい" kana="たべたい" score-mod=5 primary=KANJI-TEXT
    /// (食べ seq 10092273), words=(primary kf), score-base=NIL.
    #[test]
    fn tai1_ichidan_ren_youkei_kanji() {
        let ctx = ctx();
        let kf = kf_tai();
        let result = suffix_tai(&ctx, "食べ", "たい", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べたい");
        assert_eq!(c.kana, "たべたい");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べ");
                assert_eq!(k.seq, 10092273);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        // dict.lisp:644 — (:words (list word1 word2)) — word2 is kf wrapped.
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL TAI2: `(suffix-tai "い" "たい" kf-tai)` → NIL. The
    /// `(member root '("い") :test 'equal)` guard excludes bare い.
    #[test]
    fn tai2_i_excluded() {
        let ctx = ctx();
        let kf = kf_tai();
        let result = suffix_tai(&ctx, "い", "たい", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL TAI3: `(suffix-tai "無理" "たい" kf-tai)` → NIL. 無理 is
    /// not a verb stem; find-word-with-conj-type returns 0 rows.
    #[test]
    fn tai3_non_verb_root() {
        let ctx = ctx();
        let kf = kf_tai();
        let result = suffix_tai(&ctx, "無理", "たい", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL TAI4: `(suffix-tai "飲み" "たい" kf-tai)` → 1 COMPOUND
    /// text="飲みたい" kana="のみたい" score-mod=5 score-base=NIL
    /// primary=KANJI-TEXT (飲み seq 10665871), words=(primary kf).
    /// Exercises a godan ren'youkei stem.
    #[test]
    fn tai4_godan_ren_youkei_kanji() {
        let ctx = ctx();
        let kf = kf_tai();
        let result = suffix_tai(&ctx, "飲み", "たい", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "飲みたい");
        assert_eq!(c.kana, "のみたい");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "飲み");
                assert_eq!(k.seq, 10665871);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL TAI5: `(suffix-tai "のみ" "たい" kf-tai)` → 3 COMPOUNDs
    /// (KANA-TEXT arm of find-word-with-conj-type — three distinct
    /// kana_text rows of のみ as ren'youkei stem). Each compound has
    /// text="のみたい" kana="のみたい", a KANA-TEXT primary with
    /// text="のみ" / get-kana="のみ", and words=(primary kf). The
    /// three seqs are 10433818, 10577483, 10665871.
    #[test]
    fn tai5_kana_ren_youkei_polysemy_three() {
        let ctx = ctx();
        let kf = kf_tai();
        let result = suffix_tai(&ctx, "のみ", "たい", &kf).unwrap();
        assert_eq!(result.len(), 3);
        for c in &result {
            assert_eq!(c.text, "のみたい");
            assert_eq!(c.kana, "のみたい");
            assert!(matches!(c.score_mod, ScoreMod::Single(5)));
            assert!(c.score_base.is_none());
            match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => assert_eq!(k.text, "のみ"),
                other => panic!("expected Kana primary, got {:?}", other),
            }
            assert_eq!(c.words.len(), 2);
            match &c.words[1] {
                KaniWordDispatchEnum::Kana(k) => {
                    assert_eq!(k.seq, kf.seq);
                    assert_eq!(k.text, kf.text);
                }
                other => panic!("expected Kana word2 (kf), got {:?}", other),
            }
        }
        let mut seqs: Vec<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => unreachable!(),
            })
            .collect();
        seqs.sort();
        assert_eq!(seqs, vec![10433818, 10577483, 10665871]);
    }
}

mod suffix_ren {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:ren` suffix-cache `kf` for つつ, REPL pinned: `(get-kana-form
    /// 1008120 "つつ")` → id=1075, seq=1008120, text="つつ", common=0,
    /// common_tags="[spec1]", conjugate_p=T, nokanji=nil,
    /// best_kanji=:NULL.
    fn kf_ren_tsutsu() -> KanaText {
        KanaText {
            id: 1075,
            seq: 1008120,
            text: "つつ".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL REN1: `(suffix-ren "食べ" "つつ" kf-ren-tsutsu)` → 1
    /// COMPOUND text="食べつつ" kana="たべつつ" score-mod=5
    /// primary=KANJI-TEXT (食べ seq 10092273), words=(primary kf),
    /// score-base=NIL.
    #[test]
    fn ren1_ichidan_ren_youkei_kanji() {
        let ctx = ctx();
        let kf = kf_ren_tsutsu();
        let result = suffix_ren(&ctx, "食べ", "つつ", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べつつ");
        assert_eq!(c.kana, "たべつつ");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べ");
                assert_eq!(k.seq, 10092273);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        // dict.lisp:644 — (:words (list word1 word2)) — word2 is kf wrapped.
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL REN2: `(suffix-ren "無理" "つつ" kf-ren-tsutsu)` → NIL.
    /// 無理 has no conj-type-13 entry.
    #[test]
    fn ren2_non_verb_root() {
        let ctx = ctx();
        let kf = kf_ren_tsutsu();
        let result = suffix_ren(&ctx, "無理", "つつ", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL REN3: `(suffix-ren "い" "つつ" kf-ren-tsutsu)` → 6
    /// COMPOUNDs (suffix-ren has NO "い" gate — six conj-type-13
    /// rows for い exist as ren'youkei stems). Each compound has
    /// text="いつつ" kana="いつつ" with a KANA-TEXT primary at
    /// text="い"; pinned seqs are 2258170, 10033674, 10128912,
    /// 10303160, 10362338, 10423311.
    #[test]
    fn ren3_i_root_not_gated_six_rows() {
        let ctx = ctx();
        let kf = kf_ren_tsutsu();
        let result = suffix_ren(&ctx, "い", "つつ", &kf).unwrap();
        assert_eq!(result.len(), 6);
        for c in &result {
            assert_eq!(c.text, "いつつ");
            assert_eq!(c.kana, "いつつ");
            assert!(matches!(c.score_mod, ScoreMod::Single(5)));
            assert!(c.score_base.is_none());
            match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => assert_eq!(k.text, "い"),
                other => panic!("expected Kana primary, got {:?}", other),
            }
            assert_eq!(c.words.len(), 2);
            match &c.words[1] {
                KaniWordDispatchEnum::Kana(k) => {
                    assert_eq!(k.seq, kf.seq);
                    assert_eq!(k.text, kf.text);
                }
                other => panic!("expected Kana word2 (kf), got {:?}", other),
            }
        }
        let mut seqs: Vec<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => unreachable!(),
            })
            .collect();
        seqs.sort();
        assert_eq!(
            seqs,
            vec![2258170, 10033674, 10128912, 10303160, 10362338, 10423311]
        );
    }

    /// REPL REN4: `(suffix-ren "あり" "つつ" kf-ren-tsutsu)` → 1
    /// COMPOUND text="ありつつ" kana="ありつつ" score-mod=5
    /// score-base=NIL primary=KANA-TEXT (あり seq 2150170),
    /// words=(primary kf). Exercises the kana-text arm.
    #[test]
    fn ren4_kana_root() {
        let ctx = ctx();
        let kf = kf_ren_tsutsu();
        let result = suffix_ren(&ctx, "あり", "つつ", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "ありつつ");
        assert_eq!(c.kana, "ありつつ");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "あり");
                assert_eq!(k.seq, 2150170);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }
}

mod suffix_ren_ {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:ren-` suffix-cache `kf` for がい, REPL pinned:
    /// `(get-kana-form 2606690 "がい")` → id=177519, seq=2606690,
    /// text="がい", common=:NULL, common_tags="", conjugate_p=T,
    /// nokanji=nil, best_kanji="甲斐".
    fn kf_ren_minus_gai() -> KanaText {
        KanaText {
            id: 177519,
            seq: 2606690,
            text: "がい".into(),
            ord: 0,
            common: None,
            common_tags: String::new().into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: Some("甲斐".into()),
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL REN-1: `(suffix-ren- "食べ" "がい" kf-ren-minus-gai)` → 1
    /// COMPOUND text="食べがい" kana="たべがい" score-mod=0
    /// primary=KANJI-TEXT (食べ seq 10092273), words=(primary kf).
    #[test]
    fn ren_minus_1_ichidan_ren_youkei_kanji() {
        let ctx = ctx();
        let kf = kf_ren_minus_gai();
        let result = suffix_ren_(&ctx, "食べ", "がい", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べがい");
        assert_eq!(c.kana, "たべがい");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べ");
                assert_eq!(k.seq, 10092273);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        // dict.lisp:644 — (:words (list word1 word2)) — word2 is kf wrapped.
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL REN-2: `(suffix-ren- "無理" "がい" kf-ren-minus-gai)` → NIL.
    /// 無理 has no conj-type-13 entry.
    #[test]
    fn ren_minus_2_non_verb_root() {
        let ctx = ctx();
        let kf = kf_ren_minus_gai();
        let result = suffix_ren_(&ctx, "無理", "がい", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL REN-3: `(suffix-ren- "い" "がい" kf-ren-minus-gai)` → 6
    /// COMPOUNDs (suffix-ren- has NO "い" gate; six conj-type-13
    /// rows exist for root "い"). Each compound has text="いがい"
    /// kana="いがい" with a KANA-TEXT primary at text="い"; pinned
    /// seqs are 2258170, 10033674, 10128912, 10303160, 10362338,
    /// 10423311.
    #[test]
    fn ren_minus_3_i_root_not_gated_six_rows() {
        let ctx = ctx();
        let kf = kf_ren_minus_gai();
        let result = suffix_ren_(&ctx, "い", "がい", &kf).unwrap();
        assert_eq!(result.len(), 6);
        for c in &result {
            assert_eq!(c.text, "いがい");
            assert_eq!(c.kana, "いがい");
            assert!(matches!(c.score_mod, ScoreMod::Single(0)));
            assert!(c.score_base.is_none());
            match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => assert_eq!(k.text, "い"),
                other => panic!("expected Kana primary, got {:?}", other),
            }
            assert_eq!(c.words.len(), 2);
            match &c.words[1] {
                KaniWordDispatchEnum::Kana(k) => {
                    assert_eq!(k.seq, kf.seq);
                    assert_eq!(k.text, kf.text);
                }
                other => panic!("expected Kana word2 (kf), got {:?}", other),
            }
        }
        let mut seqs: Vec<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => unreachable!(),
            })
            .collect();
        seqs.sort();
        assert_eq!(
            seqs,
            vec![2258170, 10033674, 10128912, 10303160, 10362338, 10423311]
        );
    }
}

mod suffix_neg {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:neg` suffix-cache `kf`, REPL pinned: `(car (find-word-conj-of
    /// "なく" 1529520))` → id=1030305, seq=10648808, text="なく",
    /// common=:NULL, common_tags="", conjugate_p=nil, nokanji=nil,
    /// best_kanji=:NULL.
    fn kf_neg() -> KanaText {
        KanaText {
            id: 1030305,
            seq: 10648808,
            text: "なく".into(),
            ord: 0,
            common: None,
            common_tags: String::new().into(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL NEG1: `(suffix-neg "知ら" "なく" kf-neg)` → 1 COMPOUND
    /// text="知らなく" kana="しらなく" score-mod=5 primary=KANJI-TEXT
    /// (知ら seq 10106011), words=(primary kf). Hits conj-type 52.
    /// REPL-confirmed: (find-word-with-conj-type "知ら" 13) → 0,
    /// (… "知ら" 52) → 1.
    #[test]
    fn neg1_godan_negative_stem_kanji() {
        let ctx = ctx();
        let kf = kf_neg();
        let result = suffix_neg(&ctx, "知ら", "なく", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "知らなく");
        assert_eq!(c.kana, "しらなく");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "知ら");
                assert_eq!(k.seq, 10106011);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        // dict.lisp:644 — (:words (list word1 word2)) — word2 is kf wrapped.
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL NEG2: `(suffix-neg "食べ" "なく" kf-neg)` → 1 COMPOUND
    /// text="食べなく" kana="たべなく" score-mod=5 score-base=NIL
    /// primary=KANJI-TEXT (食べ seq 10092273), words=(primary kf).
    /// Hits conj-type 13 (ichidan ren'youkei stem) — the other end
    /// of the &[13, 52] set.
    #[test]
    fn neg2_ichidan_via_type_13() {
        let ctx = ctx();
        let kf = kf_neg();
        let result = suffix_neg(&ctx, "食べ", "なく", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べなく");
        assert_eq!(c.kana, "たべなく");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べ");
                assert_eq!(k.seq, 10092273);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL NEG3: `(suffix-neg "無理" "なく" kf-neg)` → NIL. 無理 has
    /// neither a conj-type-13 nor a conj-type-52 row.
    #[test]
    fn neg3_non_verb_root() {
        let ctx = ctx();
        let kf = kf_neg();
        let result = suffix_neg(&ctx, "無理", "なく", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL NEG4: `(suffix-neg "しら" "なく" kf-neg)` → 1 COMPOUND
    /// text="しらなく" kana="しらなく" score-mod=5 score-base=NIL
    /// primary=KANA-TEXT (しら seq 10106011), words=(primary kf).
    /// Exercises the kana-text arm of the type-52 match.
    #[test]
    fn neg4_kana_root_negative_stem() {
        let ctx = ctx();
        let kf = kf_neg();
        let result = suffix_neg(&ctx, "しら", "なく", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "しらなく");
        assert_eq!(c.kana, "しらなく");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "しら");
                assert_eq!(k.seq, 10106011);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }
}

mod te_check {
    use crate::dict::grammar::suffix::rules::*;

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL: `(te-check "で")` → NIL. Bare "で" is excluded by the
    /// first `(not (equal root "で"))` guard.
    #[test]
    fn t1_bare_de_excluded() {
        let ctx = ctx();
        let r = te_check(&ctx, "で").unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(te-check "食べる")` → NIL. Last char る not in "てで",
    /// so the second guard fails before find-word-with-conj-type runs.
    #[test]
    fn t2_last_char_not_te_or_de() {
        let ctx = ctx();
        let r = te_check(&ctx, "食べる").unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(te-check "")` signals `SIMPLE-TYPE-ERROR: Invalid index -1
    /// for (SIMPLE-ARRAY CHARACTER (0))` — `(char "" -1)` raises rather
    /// than returning. Mirror via panic.
    #[test]
    #[should_panic(
        expected = "te-check: (char root (1- (length root))) on empty root signals upstream"
    )]
    fn t3_empty_root_panics_like_upstream() {
        let ctx = ctx();
        let _ = te_check(&ctx, "");
    }

    /// REPL: `(te-check "空")` → NIL. Last char 空 not in "てで".
    #[test]
    fn t4_kanji_last_char() {
        let ctx = ctx();
        let r = te_check(&ctx, "空").unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(te-check "食べて")` → 1 word: text=食べて seq=10092233
    /// type=KANJI-TEXT wc=(92707) — conj-id 92707 is the -te form
    /// (conj-type 3) of 食べる (seq 1358280).
    #[test]
    fn t5_tabete_succeeds() {
        let ctx = ctx();
        let r = te_check(&ctx, "食べて").unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT, got {:?}", r[0]);
        };
        assert_eq!(k.text, "食べて");
        assert_eq!(k.seq, 10092233);
        assert_eq!(
            k.state.conjugations,
            Some(crate::dict::dao::WordConjugations::Ids(vec![
                92707
            ]))
        );
    }

    /// REPL: `(te-check "遊んで")` → 2 words (last char で). Verifies
    /// the で path (not just て).
    #[test]
    fn t6_asonde_de_last_char() {
        let ctx = ctx();
        let r = te_check(&ctx, "遊んで").unwrap();
        assert_eq!(r.len(), 2);
    }

    /// REPL: `(te-check "見て")` → 1 word.
    #[test]
    fn t7_mite_te_last_char() {
        let ctx = ctx();
        let r = te_check(&ctx, "見て").unwrap();
        assert_eq!(r.len(), 1);
    }
}

mod suffix_te {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:te` suffix-cache `kf` for "も", REPL pinned: `(get-kana-form
    /// 2028940 "も")` → id=110365, seq=2028940, text="も", common=0,
    /// common_tags="[spec1]", conjugate_p=T, nokanji=nil,
    /// best_kanji=:NULL.
    fn kf_te_mo() -> KanaText {
        KanaText {
            id: 110365,
            seq: 2028940,
            text: "も".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL TE1: `(suffix-te "食べて" "おる" kf-te-mo)` → 1 COMPOUND
    /// text="食べておる" kana="たべておる" score-mod=0 score-base=NIL
    /// primary=KANJI-TEXT (食べて seq 10092233). Hits the te-check
    /// te-ending arm (root last char て).
    #[test]
    fn te1_tabete_oru() {
        let ctx = ctx();
        let kf = kf_te_mo();
        let result = suffix_te(&ctx, "食べて", "おる", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べておる");
        assert_eq!(c.kana, "たべておる");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べて");
                assert_eq!(k.seq, 10092233);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL TE2: `(suffix-te "で" "おって" kf-te-mo)` → NIL. te-check's
    /// `(not (equal root "で"))` guard excludes bare で.
    #[test]
    fn te2_bare_de_excluded() {
        let ctx = ctx();
        let kf = kf_te_mo();
        let result = suffix_te(&ctx, "で", "おって", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL TE3: `(suffix-te "食べる" "おって" kf-te-mo)` → NIL. Last
    /// char る not in "てで", te-check's second guard fails.
    #[test]
    fn te3_last_char_not_te_or_de() {
        let ctx = ctx();
        let kf = kf_te_mo();
        let result = suffix_te(&ctx, "食べる", "おって", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL TE4: `(suffix-te "食べ" "おって" kf-te-mo)` → NIL. Root
    /// 食べ ends in べ; te-check's second guard fails.
    #[test]
    fn te4_stem_last_char_not_te_or_de() {
        let ctx = ctx();
        let kf = kf_te_mo();
        let result = suffix_te(&ctx, "食べ", "おって", &kf).unwrap();
        assert!(result.is_empty());
    }
}

mod teiru_check {
    use crate::dict::grammar::suffix::rules::*;

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL: `(teiru-check "いて")` → NIL. The "いて" guard excludes
    /// the literal canonical form of the iru suffix.
    #[test]
    fn t1_ite_excluded() {
        let ctx = ctx();
        let r = teiru_check(&ctx, "いて").unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(teiru-check "で")` → NIL via te-check's bare-で guard.
    #[test]
    fn t2_te_check_failure_propagates() {
        let ctx = ctx();
        let r = teiru_check(&ctx, "で").unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(teiru-check "食べる")` → NIL via te-check's "last char
    /// in てで" guard.
    #[test]
    fn t3_no_te_or_de_ending() {
        let ctx = ctx();
        let r = teiru_check(&ctx, "食べる").unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(teiru-check "食べて")` → 1 word (delegates to
    /// te-check). Same fixture as `te_check::tests::t5`.
    #[test]
    fn t4_tabete_delegates_to_te_check() {
        let ctx = ctx();
        let r = teiru_check(&ctx, "食べて").unwrap();
        assert_eq!(r.len(), 1);
        let crate::dict::kani_word::KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 10092233);
    }

    /// REPL: `(teiru-check "見て")` → 1 word.
    #[test]
    fn t5_mite() {
        let ctx = ctx();
        let r = teiru_check(&ctx, "見て").unwrap();
        assert_eq!(r.len(), 1);
    }

    /// REPL: `(teiru-check "")` signals `SIMPLE-TYPE-ERROR` via the
    /// `te-check` delegation (`(char "" -1)` raises). Mirror via panic.
    #[test]
    #[should_panic(
        expected = "te-check: (char root (1- (length root))) on empty root signals upstream"
    )]
    fn t6_empty_root_panics_via_te_check() {
        let ctx = ctx();
        let _ = teiru_check(&ctx, "");
    }
}

mod suffix_teiru {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:teiru` suffix-cache `kf` for "いる", REPL pinned via the
    /// いる(る) loop at `dict-grammar.lisp:210-215` against
    /// `(get-kana-forms 1577980)`: id=65814, seq=1577980, text="いる",
    /// common=0, common_tags="[ichi1]", conjugate_p=T, nokanji=nil,
    /// best_kanji="居る".
    fn kf_teiru_iru() -> KanaText {
        KanaText {
            id: 65814,
            seq: 1577980,
            text: "いる".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[ichi1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: Some("居る".into()),
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL TEIRU1: `(suffix-teiru "食べて" "る" kf-teiru-iru)` → 1
    /// COMPOUND text="食べてる" kana="たべてる" score-mod=3
    /// score-base=NIL primary=KANJI-TEXT (食べて seq 10092233).
    #[test]
    fn teiru1_tabete_ru() {
        let ctx = ctx();
        let kf = kf_teiru_iru();
        let result = suffix_teiru(&ctx, "食べて", "る", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べてる");
        assert_eq!(c.kana, "たべてる");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べて");
                assert_eq!(k.seq, 10092233);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL TEIRU2: `(suffix-teiru "いて" "る" kf-teiru-iru)` → NIL.
    /// teiru-check's `(not (equal root "いて"))` guard excludes bare
    /// いて.
    #[test]
    fn teiru2_ite_excluded() {
        let ctx = ctx();
        let kf = kf_teiru_iru();
        let result = suffix_teiru(&ctx, "いて", "る", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL TEIRU3: `(suffix-teiru "で" "る" kf-teiru-iru)` → NIL.
    /// teiru-check delegates to te-check whose `(not (equal root "で"))`
    /// guard fires.
    #[test]
    fn teiru3_de_excluded_via_te_check() {
        let ctx = ctx();
        let kf = kf_teiru_iru();
        let result = suffix_teiru(&ctx, "で", "る", &kf).unwrap();
        assert!(result.is_empty());
    }
}

mod suffix_teiru_plus_ {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:teiru+` suffix-cache `kf` for "いる", REPL pinned via the
    /// いる(る) loop at `dict-grammar.lisp:210-215`: id=65814,
    /// seq=1577980, text="いる", common=0, common_tags="[ichi1]",
    /// conjugate_p=T, nokanji=nil, best_kanji="居る".
    fn kf_teiru_plus_iru() -> KanaText {
        KanaText {
            id: 65814,
            seq: 1577980,
            text: "いる".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[ichi1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: Some("居る".into()),
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL TEIRU+1: `(suffix-teiru+ "食べて" "いる" kf-teiru-plus-iru)`
    /// → 1 COMPOUND text="食べている" kana="たべている" score-mod=6
    /// score-base=NIL primary=KANJI-TEXT (食べて seq 10092233).
    #[test]
    fn teiru_plus_1_tabete_iru() {
        let ctx = ctx();
        let kf = kf_teiru_plus_iru();
        let result = suffix_teiru_plus_(&ctx, "食べて", "いる", &kf)
            
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べている");
        assert_eq!(c.kana, "たべている");
        assert!(matches!(c.score_mod, ScoreMod::Single(6)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べて");
                assert_eq!(k.seq, 10092233);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL TEIRU+2: `(suffix-teiru+ "いて" "いる" kf-teiru-plus-iru)` →
    /// NIL. teiru-check's `(not (equal root "いて"))` guard excludes
    /// bare いて.
    #[test]
    fn teiru_plus_2_ite_excluded() {
        let ctx = ctx();
        let kf = kf_teiru_plus_iru();
        let result = suffix_teiru_plus_(&ctx, "いて", "いる", &kf).unwrap();
        assert!(result.is_empty());
    }
}

mod suffix_te_plus_space {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:te+space` suffix-cache `kf` for "くれる", REPL pinned via
    /// `(get-kana-form 1269130 …)`: id=33764, seq=1269130, text="くれる",
    /// common=0, common_tags="[ichi1]", conjugate_p=T, nokanji=nil,
    /// best_kanji="呉れる".
    fn kf_te_plus_space_kureru() -> KanaText {
        KanaText {
            id: 33764,
            seq: 1269130,
            text: "くれる".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[ichi1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: Some("呉れる".into()),
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL TESPACE1: `(suffix-te+space "食べて" "くれる" kf-kureru)` →
    /// 1 COMPOUND text="食べてくれる" kana="たべて くれる" score-mod=3
    /// score-base=NIL primary=KANJI-TEXT (食べて seq 10092233). Note
    /// the space between primary kana and suffix.
    #[test]
    fn tespace1_tabete_kureru() {
        let ctx = ctx();
        let kf = kf_te_plus_space_kureru();
        let result = suffix_te_plus_space(&ctx, "食べて", "くれる", &kf)
            
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べてくれる");
        assert_eq!(c.kana, "たべて くれる");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べて");
                assert_eq!(k.seq, 10092233);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL TESPACE2: `(suffix-te+space "で" "くれる" kf-kureru)` →
    /// NIL. te-check's `(not (equal root "で"))` guard fires.
    #[test]
    fn tespace2_bare_de_excluded() {
        let ctx = ctx();
        let kf = kf_te_plus_space_kureru();
        let result = suffix_te_plus_space(&ctx, "で", "くれる", &kf)
            
            .unwrap();
        assert!(result.is_empty());
    }
}

mod suffix_kudasai {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:kudasai` suffix-cache `kf` for "ください", REPL pinned via
    /// `(get-kana-form 1184270 "ください" :conj :root)`: id=25048,
    /// seq=1184270, text="ください", common=13,
    /// common_tags="[news1][nf13]", conjugate_p=T, nokanji=nil,
    /// best_kanji="下さい".
    fn kf_kudasai() -> KanaText {
        KanaText {
            id: 25048,
            seq: 1184270,
            text: "ください".into(),
            ord: 0,
            common: Some(13),
            common_tags: "[news1][nf13]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: Some("下さい".into()),
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL KUDASAI1: `(suffix-kudasai "食べて" "ください" kf-kudasai)`
    /// → 1 COMPOUND text="食べてください" kana="たべて ください"
    /// score-mod=#<FUNCTION (constantly 360)> score-base=NIL
    /// primary=KANJI-TEXT (食べて seq 10092233).
    #[test]
    fn kudasai1_tabete_kudasai() {
        let ctx = ctx();
        let kf = kf_kudasai();
        let result = suffix_kudasai(&ctx, "食べて", "ください", &kf)
            
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べてください");
        assert_eq!(c.kana, "たべて ください");
        assert!(matches!(c.score_mod, ScoreMod::Constant(360)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べて");
                assert_eq!(k.seq, 10092233);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL KUDASAI2: `(suffix-kudasai "で" "ください" kf-kudasai)` →
    /// NIL. te-check excludes bare で.
    #[test]
    fn kudasai2_bare_de_excluded() {
        let ctx = ctx();
        let kf = kf_kudasai();
        let result = suffix_kudasai(&ctx, "で", "ください", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL KUDASAI3: `(suffix-kudasai "食べる" "ください" kf-kudasai)`
    /// → NIL. te-check's last-char-in-"てで" guard fails.
    #[test]
    fn kudasai3_last_char_not_te_or_de() {
        let ctx = ctx();
        let kf = kf_kudasai();
        let result = suffix_kudasai(&ctx, "食べる", "ください", &kf)
            
            .unwrap();
        assert!(result.is_empty());
    }
}

mod suffix_te_ren {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:teren` suffix-cache `kf` for "やがって", REPL pinned via the
    /// `(load-conjs :teren 1012740 :yagaru)` populator: id=597027,
    /// seq=10285137, text="やがって", common=:NULL, common_tags="",
    /// conjugate_p=nil, nokanji=nil, best_kanji=:NULL.
    fn kf_te_ren_yagatte() -> KanaText {
        KanaText {
            id: 597027,
            seq: 10285137,
            text: "やがって".into(),
            ord: 0,
            common: None,
            common_tags: String::new().into(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL TEREN1: `(suffix-te-ren "食べて" "やがって" kf-yagatte)` →
    /// 1 COMPOUND text="食べてやがって" kana="たべてやがって"
    /// score-mod=4 score-base=NIL primary=KANJI-TEXT (食べて seq
    /// 10092233). Last char て → conj-type 3 arm.
    #[test]
    fn teren1_tabete_te_arm_conj_3() {
        let ctx = ctx();
        let kf = kf_te_ren_yagatte();
        let result = suffix_te_ren(&ctx, "食べて", "やがって", &kf)
            
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べてやがって");
        assert_eq!(c.kana, "たべてやがって");
        assert!(matches!(c.score_mod, ScoreMod::Single(4)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べて");
                assert_eq!(k.seq, 10092233);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL TEREN2: `(suffix-te-ren "食べ" "やがって" kf-yagatte)` →
    /// 1 COMPOUND text="食べやがって" kana="たべやがって" score-mod=4
    /// score-base=NIL primary=KANJI-TEXT (食べ seq 10092273). Last
    /// char べ ≠ て/で, root ≠ "い" → conj-type 13 arm (ren'youkei
    /// stem).
    #[test]
    fn teren2_tabe_stem_arm_conj_13() {
        let ctx = ctx();
        let kf = kf_te_ren_yagatte();
        let result = suffix_te_ren(&ctx, "食べ", "やがって", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べやがって");
        assert_eq!(c.kana, "たべやがって");
        assert!(matches!(c.score_mod, ScoreMod::Single(4)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べ");
                assert_eq!(k.seq, 10092273);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL TEREN3: `(suffix-te-ren "い" "やがって" kf-yagatte)` → NIL.
    /// Root "い" is excluded from the conj-type 13 arm by the
    /// `(not (member root '("い") :test 'equal))` guard, and last char
    /// い ≠ て/で so the conj-type 3 arm doesn't fire either.
    #[test]
    fn teren3_i_excluded() {
        let ctx = ctx();
        let kf = kf_te_ren_yagatte();
        let result = suffix_te_ren(&ctx, "い", "やがって", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL TEREN4: `(suffix-te-ren "で" "やがって" kf-yagatte)` → NIL.
    /// Outer `(not (equal root "で"))` guard excludes bare で.
    #[test]
    fn teren4_bare_de_excluded() {
        let ctx = ctx();
        let kf = kf_te_ren_yagatte();
        let result = suffix_te_ren(&ctx, "で", "やがって", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL TEREN5: `(suffix-te-ren "無理" "やがって" kf-yagatte)` →
    /// NIL. 無理 is not a verb stem; find-word-with-conj-type returns
    /// 0 rows for conj-type 13.
    #[test]
    fn teren5_non_verb_root() {
        let ctx = ctx();
        let kf = kf_te_ren_yagatte();
        let result = suffix_te_ren(&ctx, "無理", "やがって", &kf).unwrap();
        assert!(result.is_empty());
    }
}

mod suffix_teii {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:teii` suffix-cache `kf` for "いい", REPL pinned via
    /// `(get-kana-form 2820690 "いい")`: id=201742, seq=2820690,
    /// text="いい", common=0, common_tags="[ichi1]", conjugate_p=T,
    /// nokanji=nil, best_kanji=:NULL.
    fn kf_teii_ii() -> KanaText {
        KanaText {
            id: 201742,
            seq: 2820690,
            text: "いい".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[ichi1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL TEII1: `(suffix-teii "食べて" "いい" kf-teii-ii)` → 1
    /// COMPOUND text="食べていい" kana="たべて いい" score-mod=1
    /// score-base=NIL primary=KANJI-TEXT (食べて seq 10092233).
    #[test]
    fn teii1_tabete_ii() {
        let ctx = ctx();
        let kf = kf_teii_ii();
        let result = suffix_teii(&ctx, "食べて", "いい", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べていい");
        assert_eq!(c.kana, "たべて いい");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べて");
                assert_eq!(k.seq, 10092233);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL TEII2: `(suffix-teii "で" "いい" kf-teii-ii)` → 1 COMPOUND
    /// text="でいい" kana="で いい" score-mod=1 score-base=NIL
    /// primary=KANA-TEXT (で seq 2028980). Unlike te-check-based
    /// suffixes, suffix-teii does NOT exclude bare "で" — last char で
    /// is in "てで" and `(find-word-with-conj-type "で" 3)` returns one
    /// row.
    #[test]
    fn teii2_bare_de_not_excluded() {
        let ctx = ctx();
        let kf = kf_teii_ii();
        let result = suffix_teii(&ctx, "で", "いい", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "でいい");
        assert_eq!(c.kana, "で いい");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "で");
                assert_eq!(k.seq, 2028980);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }

    /// REPL TEII3: `(suffix-teii "食べる" "いい" kf-teii-ii)` → NIL.
    /// Last char る not in "てで", so the `(find …)` guard fails.
    #[test]
    fn teii3_last_char_not_te_or_de() {
        let ctx = ctx();
        let kf = kf_teii_ii();
        let result = suffix_teii(&ctx, "食べる", "いい", &kf).unwrap();
        assert!(result.is_empty());
    }
}

mod suffix_chau {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:chau` suffix-cache kf for "ちゃう", REPL pinned via
    /// `(postmodern:get-dao 'kana-text 108760)`: id=108760, seq=2013800,
    /// text="ちゃう", ord=0, common=0, common_tags="[spec1]",
    /// conjugate_p=T, nokanji=NIL, best_kanji=:NULL.
    fn kf_chau() -> KanaText {
        KanaText {
            id: 108760,
            seq: 2013800,
            text: "ちゃう".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    /// `:chau` cache kf for "じゃう", REPL pinned via
    /// `(postmodern:get-dao 'kana-text 108761)`: id=108761, seq=2013800,
    /// text="じゃう", ord=1, common=0, common_tags="[spec1]",
    /// conjugate_p=T, nokanji=NIL, best_kanji=:NULL.
    fn kf_jau() -> KanaText {
        KanaText {
            id: 108761,
            seq: 2013800,
            text: "じゃう".into(),
            ord: 1,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL CHAU1: `(suffix-chau "食べ" "ちゃう" kf-chau)` → 1 COMPOUND
    /// text="食べちゃう" kana="たべちゃう" score-mod=5 score-base=NIL
    /// primary=KANJI-TEXT (食べて id=411243 seq=10092233),
    /// words=(primary, kf-chau). Exercises the ち→て arm.
    #[test]
    fn chau1_ti_arm_kanji() {
        let ctx = ctx();
        let kf = kf_chau();
        let result = suffix_chau(&ctx, "食べ", "ちゃう", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べちゃう");
        assert_eq!(c.kana, "たべちゃう");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 411243);
                assert_eq!(k.seq, 10092233);
                assert_eq!(k.text, "食べて");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL CHAU2: `(suffix-chau "読ん" "じゃう" kf-jau)` → 1 COMPOUND
    /// text="読んじゃう" kana="よんじゃう" score-mod=5 score-base=NIL
    /// primary=KANJI-TEXT (読んで id=431719 seq=10102130),
    /// words=(primary, kf-jau). Exercises the じ→で arm.
    #[test]
    fn chau2_zi_arm_kanji() {
        let ctx = ctx();
        let kf = kf_jau();
        let result = suffix_chau(&ctx, "読ん", "じゃう", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "読んじゃう");
        assert_eq!(c.kana, "よんじゃう");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 431719);
                assert_eq!(k.seq, 10102130);
                assert_eq!(k.text, "読んで");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        // adjoin_word puts word1 at words[0] (dict.lisp:644 — `(list word1 word2)`).
        assert_eq!(c.words.len(), 2);
        match &c.words[0] {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.id, 431719),
            other => panic!("expected Kanji words[0] (primary), got {:?}", other),
        }
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL CHAU3: `(suffix-chau "食べ" "あう" kf-chau)` → NIL. First
    /// char あ is neither じ nor ち, so the `case` returns NIL and the
    /// outer `when te` guard suppresses the lookup.
    #[test]
    fn chau3_other_first_char() {
        let ctx = ctx();
        let kf = kf_chau();
        let result = suffix_chau(&ctx, "食べ", "あう", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL CHAU4: `(suffix-chau "食べ" "じゃう" kf-jau)` → NIL.
    /// じ→で arm picks "で", but "食べで" is not a conj-type-3 form
    /// (only "食べて" is), so `find-word-with-conj-type` returns NIL.
    #[test]
    fn chau4_de_arm_no_match() {
        let ctx = ctx();
        let kf = kf_jau();
        let result = suffix_chau(&ctx, "食べ", "じゃう", &kf).unwrap();
        assert!(result.is_empty());
    }
}

mod suffix_to {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:to` cache kf for "とく", REPL pinned via
    /// `(postmodern:get-dao 'kana-text 119112)`: id=119112, seq=2108590,
    /// text="とく", ord=0, common=0, common_tags="[spec1]",
    /// conjugate_p=T, nokanji=NIL, best_kanji=:NULL.
    fn kf_toku() -> KanaText {
        KanaText {
            id: 119112,
            seq: 2108590,
            text: "とく".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    /// `:to` cache kf for "どく", REPL pinned via
    /// `(postmodern:get-dao 'kana-text 119113)`: id=119113, seq=2108590,
    /// text="どく", ord=1, common=:NULL, common_tags="",
    /// conjugate_p=T, nokanji=NIL, best_kanji=:NULL.
    fn kf_doku() -> KanaText {
        KanaText {
            id: 119113,
            seq: 2108590,
            text: "どく".into(),
            ord: 1,
            common: None,
            common_tags: String::new().into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL TO1: `(suffix-to "食べ" "とく" kf-toku)` → 1 COMPOUND
    /// text="食べとく" kana="たべとく" score-mod=0 score-base=NIL
    /// primary=KANJI-TEXT (食べて id=411243 seq=10092233),
    /// words=(primary, kf-toku). Exercises the と→て arm.
    #[test]
    fn to1_to_arm_kanji() {
        let ctx = ctx();
        let kf = kf_toku();
        let result = suffix_to(&ctx, "食べ", "とく", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べとく");
        assert_eq!(c.kana, "たべとく");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 411243);
                assert_eq!(k.seq, 10092233);
                assert_eq!(k.text, "食べて");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL TO2: `(suffix-to "読ん" "どく" kf-doku)` → 1 COMPOUND
    /// text="読んどく" kana="よんどく" score-mod=0 score-base=NIL
    /// primary=KANJI-TEXT (読んで id=431719 seq=10102130),
    /// words=(primary, kf-doku). Exercises the ど→で arm.
    #[test]
    fn to2_do_arm_kanji() {
        let ctx = ctx();
        let kf = kf_doku();
        let result = suffix_to(&ctx, "読ん", "どく", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "読んどく");
        assert_eq!(c.kana, "よんどく");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 431719);
                assert_eq!(k.seq, 10102130);
                assert_eq!(k.text, "読んで");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        // adjoin_word puts word1 at words[0] (dict.lisp:644 — `(list word1 word2)`).
        assert_eq!(c.words.len(), 2);
        match &c.words[0] {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.id, 431719),
            other => panic!("expected Kanji words[0] (primary), got {:?}", other),
        }
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL TO3: `(suffix-to "食べ" "あく" kf-toku)` → NIL. First char
    /// あ is neither と nor ど, so the `case` returns NIL.
    #[test]
    fn to3_other_first_char() {
        let ctx = ctx();
        let kf = kf_toku();
        let result = suffix_to(&ctx, "食べ", "あく", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL TO4: `(suffix-to "のん" "どく" kf-doku)` → 3 COMPOUNDs
    /// (kana-text arm of find-word-with-conj-type — three distinct
    /// kana_text のんで rows). Each compound has text="のんどく"
    /// kana="のんどく", KANA-TEXT primary with text "のんで". Seqs:
    /// 10433774, 10577439, 10665827; ids: 773379, 945133, 1050587.
    #[test]
    fn to4_polysemy_kana_three() {
        let ctx = ctx();
        let kf = kf_doku();
        let result = suffix_to(&ctx, "のん", "どく", &kf).unwrap();
        assert_eq!(result.len(), 3);
        for c in &result {
            assert_eq!(c.text, "のんどく");
            assert_eq!(c.kana, "のんどく");
            assert!(matches!(c.score_mod, ScoreMod::Single(0)));
            assert!(c.score_base.is_none());
            match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => assert_eq!(k.text, "のんで"),
                other => panic!("expected Kana primary, got {:?}", other),
            }
            assert_eq!(c.words.len(), 2);
            match &c.words[1] {
                KaniWordDispatchEnum::Kana(k) => {
                    assert_eq!(k.seq, kf.seq);
                    assert_eq!(k.text, kf.text);
                }
                other => panic!("expected Kana word2 (kf), got {:?}", other),
            }
        }
        let mut got: Vec<(i32, i32)> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => (k.id, k.seq),
                _ => unreachable!(),
            })
            .collect();
        got.sort();
        assert_eq!(
            got,
            vec![(773379, 10433774), (945133, 10577439), (1050587, 10665827)]
        );
    }
}

mod suffix_suru {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::dao::SimpleText;

    /// Construct the `:suru` suffix-cache `kf` REPL pinned for the
    /// test corpus: id=439727, seq=10152292, text="し",
    /// conjugate_p=nil, nokanji=nil, best_kanji=:NULL, conjugations
    /// referencing seq 153220, hintedp=nil. Pulled verbatim from
    /// `corpus/extracted_chunk_c_suffix_abbr_2026_05_16/dict/\
    /// suffix_suru.parquet` row 0.
    fn kf_suru() -> KanaText {
        KanaText {
            id: 439727,
            seq: 10152292,
            text: "し".into(),
            ord: 0,
            common: None,
            common_tags: String::new().into(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL T1: `(suffix-suru "区別" "し" kf-suru)` → 1 COMPOUND
    /// text="区別し" kana="くべつ し" score-mod=5 primary=KANJI-TEXT
    /// (区別 seq 1244250).
    #[test]
    fn t1_kanji_root_with_vs_pos() {
        let ctx = ctx();
        let kf = kf_suru();
        let result = suffix_suru(&ctx, "区別", "し", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "区別し");
        assert_eq!(c.kana, "くべつ し");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "区別");
                assert_eq!(k.seq, 1244250);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
    }

    /// REPL T2: `(suffix-suru "青空" "し" kf-suru)` → 0 (青空 has no
    /// `vs` pos in `sense_prop`).
    #[test]
    fn t2_kanji_root_no_vs_match() {
        let ctx = ctx();
        let kf = kf_suru();
        let result = suffix_suru(&ctx, "青空", "し", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL T3: `(suffix-suru "ジョギング" "し" kf-suru)` → 1 COMPOUND
    /// text="ジョギングし" kana="ジョギング し" score-mod=5
    /// score-base=NIL primary=KANA-TEXT (ジョギング seq 1066360),
    /// words=(primary kf). Exercises the kana-text dispatch arm of
    /// `find-word-with-pos` (pure-katakana input).
    #[test]
    fn t3_katakana_root_kana_text_arm() {
        let ctx = ctx();
        let kf = kf_suru();
        let result = suffix_suru(&ctx, "ジョギング", "し", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "ジョギングし");
        assert_eq!(c.kana, "ジョギング し");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "ジョギング");
                assert_eq!(k.seq, 1066360);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL T4: `(suffix-suru "" "し" kf-suru)` → 0 (empty root never
    /// matches any kanji_text/kana_text row).
    #[test]
    fn t4_empty_root() {
        let ctx = ctx();
        let kf = kf_suru();
        let result = suffix_suru(&ctx, "", "し", &kf).unwrap();
        assert!(result.is_empty());
    }
}

mod apply_patch {
    use crate::dict::grammar::suffix::rules::*;

    /// REPL: `(ichiran/dict::apply-patch "なさ" (cons "い" "さ"))` → `"ない"`.
    /// Mirrors the `suffix-sugiru` rewrite of a "なさ" / "無さ" tail.
    #[test]
    fn replaces_trailing_sa_with_i() {
        assert_eq!(apply_patch("なさ", ("い", "さ")), "ない");
    }

    /// REPL: `(ichiran/dict::apply-patch "そ" (cons "う" ""))` → `"そう"`.
    /// Mirrors the `suffix-garu` "そ" → "そう" promotion (empty removed).
    #[test]
    fn empty_removed_appends_replacement() {
        assert_eq!(apply_patch("そ", ("う", "")), "そう");
    }

    /// REPL: `(ichiran/dict::apply-patch "あいうえお" (cons "XX" "えお"))` →
    /// `"あいうXX"` (multi-character removal across multi-byte UTF-8).
    #[test]
    fn multi_char_removal_multi_byte() {
        assert_eq!(apply_patch("あいうえお", ("XX", "えお")), "あいうXX");
    }

    /// REPL: `(ichiran/dict::apply-patch "abc" (cons "" "abc"))` → `""`
    /// (entire root is the removed tail, replacement is empty).
    #[test]
    fn full_removal_empty_replacement_yields_empty() {
        assert_eq!(apply_patch("abc", ("", "abc")), "");
    }

    /// REPL: `(ichiran/dict::apply-patch "abc" (cons "" ""))` → `"abc"`
    /// (no-op patch returns the root unchanged).
    #[test]
    fn empty_patch_returns_root_unchanged() {
        assert_eq!(apply_patch("abc", ("", "")), "abc");
    }

    /// REPL: `(length (ichiran/dict::apply-patch "abc" (cons "" "")))` → `3`.
    /// Length pin to verify char counts match the upstream `simple-array
    /// character (3)` return shape.
    #[test]
    fn output_char_length_matches_upstream() {
        let out = apply_patch("abc", ("", ""));
        assert_eq!(out.chars().count(), 3);
    }

    /// REPL: `(apply-patch "abc" (cons "x" "toolong"))` →
    /// `ERROR: The value -4 is not of type (OR (MOD …) NULL) when
    /// binding SB-IMPL::END`. Pins that the Rust port also rejects
    /// removed > root rather than silently wrapping `usize`. The
    /// upstream message text is SBCL-internal and not asserted.
    #[test]
    #[should_panic]
    fn removed_longer_than_root_panics() {
        let _ = apply_patch("abc", ("x", "toolong"));
    }
}

mod suffix_sou {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:sou` suffix-cache `kf` for "そう", REPL pinned via
    /// `(gethash "そう" *suffix-cache*)`: id=876, seq=1006610, text="そう",
    /// ord=0, common=0, common_tags="[ichi1]", conjugate_p=T,
    /// nokanji=NIL, best_kanji=:NULL.
    fn kf_sou() -> KanaText {
        KanaText {
            id: 876,
            seq: 1006610,
            text: "そう".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[ichi1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL: `(suffix-sou "美味し" "そう" kf-sou)` → 1 COMPOUND
    /// text="美味しそう" kana="おいしそう" score-mod=(constantly 70)
    /// primary=KANJI-TEXT (美味し id=1433173 seq=10597564), patch=nil,
    /// words=(primary, kf). Exercises the catch-all `(t 70)` arm.
    #[test]
    fn sou1_adj_stem_kanji_score70() {
        let ctx = ctx();
        let kf = kf_sou();
        let result = suffix_sou(&ctx, "美味し", "そう", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "美味しそう");
        assert_eq!(c.kana, "おいしそう");
        assert!(matches!(c.score_mod, ScoreMod::Constant(70)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 1433173);
                assert_eq!(k.seq, 10597564);
                assert_eq!(k.text, "美味し");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
    }

    /// REPL: `(suffix-sou "出来" "そう" kf-sou)` → 1 COMPOUND
    /// text="出来そう" kana="できそう" score-mod=(constantly 100)
    /// primary=KANJI-TEXT (出来 id=689432 seq=10230657). Pins the
    /// `((equal root "出来") 100)` arm.
    #[test]
    fn sou2_dekiru_arm_score100() {
        let ctx = ctx();
        let kf = kf_sou();
        let result = suffix_sou(&ctx, "出来", "そう", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "出来そう");
        assert_eq!(c.kana, "できそう");
        assert!(matches!(c.score_mod, ScoreMod::Constant(100)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10230657),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sou "から" "そう" kf-sou)` → 2 COMPOUNDs
    /// (text="からそう" each), both with score-mod=(constantly 40)
    /// (the `((equal root "から") 40)` arm). Primary seqs 2858914 / 10419670.
    #[test]
    fn sou3_kara_arm_score40() {
        let ctx = ctx();
        let kf = kf_sou();
        let result = suffix_sou(&ctx, "から", "そう", &kf).unwrap();
        assert_eq!(result.len(), 2);
        for c in &result {
            assert_eq!(c.text, "からそう");
            assert!(matches!(c.score_mod, ScoreMod::Constant(40)));
            assert!(c.score_base.is_none());
            assert_eq!(c.words.len(), 2);
        }
        let seqs: Vec<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => -1,
            })
            .collect();
        assert!(seqs.contains(&2858914));
        assert!(seqs.contains(&10419670));
    }

    /// REPL: `(suffix-sou "い" "そう" kf-sou)` → 6 COMPOUNDs
    /// text="いそう" each, score-mod=(constantly 0). Hits the
    /// `((equal root "い") 0)` arm and finds 6 い-rooted conj-stem rows.
    #[test]
    fn sou4_i_arm_score0() {
        let ctx = ctx();
        let kf = kf_sou();
        let result = suffix_sou(&ctx, "い", "そう", &kf).unwrap();
        assert_eq!(result.len(), 6);
        for c in &result {
            assert_eq!(c.text, "いそう");
            assert_eq!(c.kana, "いそう");
            assert!(matches!(c.score_mod, ScoreMod::Constant(0)));
            assert!(c.score_base.is_none());
            assert_eq!(c.words.len(), 2);
        }
    }

    /// REPL: `(suffix-sou "な" "そう" kf-sou)` → NIL — `root` is in the
    /// `'("な" "よ" "よさ" "に" "き")` exclusion list AND doesn't end
    /// with "なさ", so suffix-sou-base's cond falls through to nil.
    #[test]
    fn sou5_excluded_root_returns_empty() {
        let ctx = ctx();
        let kf = kf_sou();
        for r in ["な", "よ", "よさ", "に", "き"] {
            let result = suffix_sou(&ctx, r, "そう", &kf).unwrap();
            assert!(result.is_empty(), "expected NIL for root={:?}", r);
        }
    }

    /// REPL: `(suffix-sou "つまらなさ" "そう" kf-sou)` → 1 COMPOUND
    /// text="つまらなさそう" kana="つまらなさそう" smod=(constantly 70)
    /// primary=KANA-TEXT (つまらない id=1082 seq=1008190). Exercises the
    /// "なさ"-tail branch: patch=("い","さ") rewrites root to "つまらない",
    /// find-word-with-conj-prop with conj-neg filter returns 1 row, and
    /// the kana branch uses `destem(k, length("い")=1) + "さ" + suf`.
    #[test]
    fn sou6_nasa_branch_kana() {
        let ctx = ctx();
        let kf = kf_sou();
        let result = suffix_sou(&ctx, "つまらなさ", "そう", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "つまらなさそう");
        assert_eq!(c.kana, "つまらなさそう");
        assert!(matches!(c.score_mod, ScoreMod::Constant(70)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.id, 1082);
                assert_eq!(k.seq, 1008190);
                assert_eq!(k.text, "つまらない");
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sou "食べなさ" "そう" kf-sou)` → 1 COMPOUND
    /// text="食べなさそう" kana="たべなさそう" primary=KANJI-TEXT
    /// (食べない id=411231 seq=10092227). Pins the "なさ" branch on a
    /// kanji-text result.
    #[test]
    fn sou7_nasa_branch_kanji() {
        let ctx = ctx();
        let kf = kf_sou();
        let result = suffix_sou(&ctx, "食べなさ", "そう", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べなさそう");
        assert_eq!(c.kana, "たべなさそう");
        assert!(matches!(c.score_mod, ScoreMod::Constant(70)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 411231);
                assert_eq!(k.seq, 10092227);
                assert_eq!(k.text, "食べない");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }
}

mod suffix_sou_plus_ {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:sou+` cache `kf` for "そう". The :sou+ entry shares the cache
    /// row text "そう" with :sou (the `(load-conjs :sou+ 2141080)`
    /// callsite at `dict-grammar.lisp:251` loads conjugations of
    /// そうにない / そうにありません; each load-kf overwrites the
    /// `text -> (key kf)` slot without `:join t`). The :sou+ key is
    /// observable in the cache for the conjugated forms; the base
    /// suffix kf used at runtime is the cache row registered against
    /// "そう". Pinned from REPL (cache row at id=876 seq=1006610).
    fn kf_sou_plus_() -> KanaText {
        KanaText {
            id: 876,
            seq: 1006610,
            text: "そう".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[ichi1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL: `(suffix-sou+ "美味し" "そうにない" kf)` → 1 COMPOUND
    /// text="美味しそうにない" kana="おいしそうにない" score-mod=1
    /// primary=KANJI-TEXT (美味し id=1433173 seq=10597564). Same body
    /// as suffix-sou's catch-all arm, but with the literal `:score 1`.
    #[test]
    fn sou_plus_1_adj_stem_kanji() {
        let ctx = ctx();
        let kf = kf_sou_plus_();
        let result = suffix_sou_plus_(&ctx, "美味し", "そうにない", &kf)
            
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "美味しそうにない");
        assert_eq!(c.kana, "おいしそうにない");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 1433173);
                assert_eq!(k.seq, 10597564);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sou+ "出来" "そうにない" kf)` → 1 COMPOUND
    /// text="出来そうにない" kana="できそうにない" score-mod=1
    /// primary=KANJI-TEXT (出来 seq 10230657). Exercises the
    /// conj-adj-stem arm with a different root.
    #[test]
    fn sou_plus_2_dekiru() {
        let ctx = ctx();
        let kf = kf_sou_plus_();
        let result = suffix_sou_plus_(&ctx, "出来", "そうにない", &kf)
            
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "出来そうにない");
        assert_eq!(c.kana, "できそうにない");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10230657),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sou+ "つまらなさ" "そう" kf)` → 1 COMPOUND
    /// text="つまらなさそう" kana="つまらなさそう" score-mod=1
    /// primary=KANA-TEXT (つまらない id=1082 seq=1008190). Pins the
    /// "なさ"-tail branch path through suffix-sou-base with `:score 1`.
    #[test]
    fn sou_plus_3_nasa_branch() {
        let ctx = ctx();
        let kf = kf_sou_plus_();
        let result = suffix_sou_plus_(&ctx, "つまらなさ", "そう", &kf)
            
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "つまらなさそう");
        assert_eq!(c.kana, "つまらなさそう");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.id, 1082);
                assert_eq!(k.seq, 1008190);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sou+ "な" "そうにない" kf)` → NIL — `root` "な" is
    /// in the exclusion list `'("な" "よ" "よさ" "に" "き")`.
    #[test]
    fn sou_plus_4_excluded_root() {
        let ctx = ctx();
        let kf = kf_sou_plus_();
        let result = suffix_sou_plus_(&ctx, "な", "そうにない", &kf)
            
            .unwrap();
        assert!(result.is_empty());
    }
}

mod suffix_rou {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:rou` suffix-cache `kf`, REPL pinned: `(get-kana-form 1928670
    /// "だろう")` → id=99986, seq=1928670, text="だろう", common=0,
    /// common_tags="[spec1]", conjugate_p=T, nokanji=nil, best_kanji=:NULL.
    /// The cache key is the `:text` override `"ろう"`; the kf object
    /// itself carries `"だろう"`.
    fn kf_rou() -> KanaText {
        KanaText {
            id: 99986,
            seq: 1928670,
            text: "だろう".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL ROU1: `(suffix-rou "食べた" "ろう" kf-rou)` → 1 COMPOUND
    /// text="食べたろう" kana="たべたろう" score-mod=1 score-base=NIL
    /// primary=KANJI-TEXT (食べた seq 10092229), words=(primary kf).
    /// Exercises the past-plain (conj-type 2) kanji arm.
    #[test]
    fn rou1_ichidan_past_kanji() {
        let ctx = ctx();
        let kf = kf_rou();
        let result = suffix_rou(&ctx, "食べた", "ろう", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べたろう");
        assert_eq!(c.kana, "たべたろう");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べた");
                assert_eq!(k.seq, 10092229);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL ROU2: `(suffix-rou "なかった" "ろう" kf-rou)` → 4 COMPOUNDs
    /// — four conj-type-2 kana-text rows for なかった. Each has
    /// text="なかったろう" kana="なかったろう" score-mod=1 primary=KANA.
    /// Pinned seqs: 10076179, 10470716, 10517041, 10648797.
    #[test]
    fn rou2_nakatta_polysemy_four() {
        let ctx = ctx();
        let kf = kf_rou();
        let result = suffix_rou(&ctx, "なかった", "ろう", &kf).unwrap();
        assert_eq!(result.len(), 4);
        for c in &result {
            assert_eq!(c.text, "なかったろう");
            assert_eq!(c.kana, "なかったろう");
            assert!(matches!(c.score_mod, ScoreMod::Single(1)));
            assert!(c.score_base.is_none());
            match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => assert_eq!(k.text, "なかった"),
                other => panic!("expected Kana primary, got {:?}", other),
            }
            assert_eq!(c.words.len(), 2);
        }
        let mut seqs: Vec<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => unreachable!(),
            })
            .collect();
        seqs.sort();
        assert_eq!(seqs, vec![10076179, 10470716, 10517041, 10648797]);
    }

    /// REPL ROU3: `(suffix-rou "食べる" "ろう" kf-rou)` → NIL. 食べる is
    /// a root form (conj-type :root), not past-plain (conj-type 2);
    /// find-word-with-conj-type returns 0 rows.
    #[test]
    fn rou3_root_form_no_match() {
        let ctx = ctx();
        let kf = kf_rou();
        let result = suffix_rou(&ctx, "食べる", "ろう", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL ROU4: `(suffix-rou "無理" "ろう" kf-rou)` → NIL. Non-verb
    /// root.
    #[test]
    fn rou4_non_verb_root() {
        let ctx = ctx();
        let kf = kf_rou();
        let result = suffix_rou(&ctx, "無理", "ろう", &kf).unwrap();
        assert!(result.is_empty());
    }
}

mod suffix_adv {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:adv` suffix-cache `kf` for "なる" — the root kana-text of seq
    /// 1375610. REPL pinned: id=44705, seq=1375610, text="なる",
    /// common=34, common_tags="[ichi1][news2][nf34]", conjugate_p=T,
    /// nokanji=nil, best_kanji="成る". The `(load-conjs :adv 1375610
    /// :naru)` loader walks every kana-form of 1375610 (211 rows); we
    /// pick the root row as a representative.
    fn kf_adv_naru() -> KanaText {
        KanaText {
            id: 44705,
            seq: 1375610,
            text: "なる".into(),
            ord: 0,
            common: Some(34),
            common_tags: "[ichi1][news2][nf34]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: Some("成る".into()),
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL ADV1: `(suffix-adv "正しく" "なる" kf-adv-naru)` → 1
    /// COMPOUND text="正しくなる" kana="ただしくなる" score-mod=1
    /// score-base=NIL primary=KANJI-TEXT (正しく seq 2827272),
    /// words=(primary kf).
    #[test]
    fn adv1_tadashiku_naru() {
        let ctx = ctx();
        let kf = kf_adv_naru();
        let result = suffix_adv(&ctx, "正しく", "なる", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "正しくなる");
        assert_eq!(c.kana, "ただしくなる");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "正しく");
                assert_eq!(k.seq, 2827272);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL ADV2: `(suffix-adv "大きく" "なる" kf-adv-naru)` → 1
    /// COMPOUND text="大きくなる" kana="おおきくなる" score-mod=1
    /// primary=KANJI-TEXT (大きく seq 10563301).
    #[test]
    fn adv2_ookiku_naru() {
        let ctx = ctx();
        let kf = kf_adv_naru();
        let result = suffix_adv(&ctx, "大きく", "なる", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "大きくなる");
        assert_eq!(c.kana, "おおきくなる");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "大きく");
                assert_eq!(k.seq, 10563301);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL ADV3: `(suffix-adv "無理" "なる" kf-adv-naru)` → NIL.
    /// 無理 has no conj-type-50 row.
    #[test]
    fn adv3_non_adverbial_root() {
        let ctx = ctx();
        let kf = kf_adv_naru();
        let result = suffix_adv(&ctx, "無理", "なる", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL ADV4: `(suffix-adv "ジャバスクリプト" "なる" kf-adv-naru)` →
    /// NIL. Word with no conjugations.
    #[test]
    fn adv4_no_conjugation_root() {
        let ctx = ctx();
        let kf = kf_adv_naru();
        let result = suffix_adv(&ctx, "ジャバスクリプト", "なる", &kf)
            
            .unwrap();
        assert!(result.is_empty());
    }
}

mod suffix_sugiru {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::dao::SimpleText;

    /// `:sugiru` cache `kf` for "すぎる", REPL pinned: id=26253,
    /// seq=1195970, text="すぎる", ord=0, common=34,
    /// common_tags="[ichi1][news2][nf34]", conjugate_p=T, nokanji=NIL,
    /// best_kanji=:NULL.
    fn kf_sugiru() -> KanaText {
        KanaText {
            id: 26253,
            seq: 1195970,
            text: "すぎる".into(),
            ord: 0,
            common: Some(34),
            common_tags: "[ichi1][news2][nf34]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL: `(suffix-sugiru "高" "すぎる" kf)` → 1 COMPOUND
    /// text="高すぎる" kana="たかすぎる" score-mod=5 primary=KANJI-TEXT
    /// (高い id=18690 seq=1283190). Exercises the `(t (concatenate root "い"))`
    /// branch → find-word-with-pos "高い" "adj-i". kana="たかい",
    /// destem(kana,1)="たか", + "" + "すぎる" = "たかすぎる".
    #[test]
    fn sugiru1_adj_i_short_root() {
        let ctx = ctx();
        let kf = kf_sugiru();
        let result = suffix_sugiru(&ctx, "高", "すぎる", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "高すぎる");
        assert_eq!(c.kana, "たかすぎる");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 18690);
                assert_eq!(k.seq, 1283190);
                assert_eq!(k.text, "高い");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sugiru "つまらな" "すぎる" kf)` → 1 COMPOUND
    /// text="つまらなすぎる" kana="つまらなすぎる" primary=KANA-TEXT
    /// (つまらない seq 1008190). Else-branch (no patch): new-root
    /// "つまらない", find-word-with-pos "adj-i".
    #[test]
    fn sugiru2_adj_i_kana_root() {
        let ctx = ctx();
        let kf = kf_sugiru();
        let result = suffix_sugiru(&ctx, "つまらな", "すぎる", &kf)
            
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "つまらなすぎる");
        assert_eq!(c.kana, "つまらなすぎる");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.id, 1082);
                assert_eq!(k.seq, 1008190);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sugiru "つまらなさ" "すぎる" kf)` → 1 COMPOUND
    /// text="つまらなさすぎる" kana="つまらなさすぎる" primary=KANA-TEXT
    /// (つまらない seq 1008190). Patch branch (length new-root=5 > 2):
    /// patch=("い","さ"), new-root="つまらない", find-word-with-conj-prop
    /// conj-neg → 1 row. Kana=destem("つまらない",1)+"さ"+""+"すぎる" =
    /// "つまらな"+"さ"+"すぎる".
    #[test]
    fn sugiru3_nasa_tail_long_conj_prop_branch() {
        let ctx = ctx();
        let kf = kf_sugiru();
        let result = suffix_sugiru(&ctx, "つまらなさ", "すぎる", &kf)
            
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "つまらなさすぎる");
        assert_eq!(c.kana, "つまらなさすぎる");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => assert_eq!(k.seq, 1008190),
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sugiru "無さ" "すぎる" kf)` → 1 COMPOUND
    /// text="無さすぎる" kana="なさすぎる" primary=KANJI-TEXT
    /// (無い id=49726 seq=1529520). Patch branch falls through because
    /// length new-root=2 ≤ 2 → find-word-with-pos "無い" "adj-i".
    /// Kana=destem("ない",1)+"さ"+""+"すぎる"="な"+"さ"+"すぎる".
    #[test]
    fn sugiru4_nasa_kanji_short_falls_to_pos() {
        let ctx = ctx();
        let kf = kf_sugiru();
        let result = suffix_sugiru(&ctx, "無さ", "すぎる", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "無さすぎる");
        assert_eq!(c.kana, "なさすぎる");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 49726);
                assert_eq!(k.seq, 1529520);
                assert_eq!(k.text, "無い");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sugiru "無" "すぎる" kf)` → 1 COMPOUND
    /// text="無すぎる" kana="なすぎる" primary=KANJI-TEXT (無い seq 1529520).
    /// Else-branch (no patch): new-root "無い".
    #[test]
    fn sugiru5_kanji_short_else_branch() {
        let ctx = ctx();
        let kf = kf_sugiru();
        let result = suffix_sugiru(&ctx, "無", "すぎる", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "無すぎる");
        assert_eq!(c.kana, "なすぎる");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 1529520),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sugiru "美味し" "すぎる" kf)` → 1 COMPOUND
    /// text="美味しすぎる" kana="おいしすぎる" primary=KANJI-TEXT
    /// (美味しい id=44494 seq=1486650).
    #[test]
    fn sugiru6_oishii() {
        let ctx = ctx();
        let kf = kf_sugiru();
        let result = suffix_sugiru(&ctx, "美味し", "すぎる", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "美味しすぎる");
        assert_eq!(c.kana, "おいしすぎる");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 44494);
                assert_eq!(k.seq, 1486650);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sugiru "い" "すぎる" kf)` → NIL — first-branch
    /// `((equal root "い") nil)` short-circuits the outer `when root`.
    #[test]
    fn sugiru7_i_root_returns_nil() {
        let ctx = ctx();
        let kf = kf_sugiru();
        let result = suffix_sugiru(&ctx, "い", "すぎる", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL: `(suffix-sugiru "食べ" "すぎる" kf)` and `"やり"` and `"行か"`
    /// all → NIL — else-branch new-root ("食べい"/"やりい"/"行かい") is not
    /// an adj-i lemma.
    #[test]
    fn sugiru8_non_adj_else_returns_empty() {
        let ctx = ctx();
        let kf = kf_sugiru();
        for r in ["食べ", "やり", "行か"] {
            let result = suffix_sugiru(&ctx, r, "すぎる", &kf).unwrap();
            assert!(result.is_empty(), "expected NIL for root={:?}", r);
        }
    }
}

mod suffix_sa {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::dao::SimpleText;

    /// `:sa` suffix-cache kf for "さ", REPL pinned via
    /// `(postmodern:get-dao 'kana-text 110392)`: id=110392, seq=2029120,
    /// text="さ", ord=0, common=0, common_tags="[spec1]",
    /// conjugate_p=T, nokanji=NIL, best_kanji=:NULL.
    fn kf_sa() -> KanaText {
        KanaText {
            id: 110392,
            seq: 2029120,
            text: "さ".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL SA1: `(suffix-sa "美し" "さ" kf-sa)` → 1 COMPOUND
    /// text="美しさ" kana="うつくしさ" score-mod=2 score-base=NIL
    /// primary=KANJI-TEXT (美し id=263320 seq=10017294),
    /// words=(primary, kf-sa). Exercises arm A (conj-type 51) only.
    #[test]
    fn sa1_adj_i_stem_kanji() {
        let ctx = ctx();
        let kf = kf_sa();
        let result = suffix_sa(&ctx, "美し", "さ", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "美しさ");
        assert_eq!(c.kana, "うつくしさ");
        assert!(matches!(c.score_mod, ScoreMod::Single(2)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 263320);
                assert_eq!(k.seq, 10017294);
                assert_eq!(k.text, "美し");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL SA2: `(suffix-sa "静か" "さ" kf-sa)` → 1 COMPOUND
    /// text="静かさ" kana="しずかさ" score-mod=2 score-base=NIL
    /// primary=KANJI-TEXT (静か id=31238 seq=1381820),
    /// words=(primary, kf-sa). Exercises arm B (adj-na) only.
    #[test]
    fn sa2_adj_na_kanji() {
        let ctx = ctx();
        let kf = kf_sa();
        let result = suffix_sa(&ctx, "静か", "さ", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "静かさ");
        assert_eq!(c.kana, "しずかさ");
        assert!(matches!(c.score_mod, ScoreMod::Single(2)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 31238);
                assert_eq!(k.seq, 1381820);
                assert_eq!(k.text, "静か");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        // adjoin_word puts word1 at words[0] (dict.lisp:644 — `(list word1 word2)`).
        assert_eq!(c.words.len(), 2);
        match &c.words[0] {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.id, 31238),
            other => panic!("expected Kanji words[0] (primary), got {:?}", other),
        }
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL SA3: `(suffix-sa "やわらか" "さ" kf-sa)` → 2 COMPOUNDs
    /// (one from each arm, both KANA-TEXT). Arm A: id=1018986
    /// seq=10639355. Arm B: id=53460 seq=1460730. Both text="やわらか",
    /// kana="やわらかさ". Exercises the nconc concatenation order
    /// (arm A before arm B).
    #[test]
    fn sa3_both_arms_kana_yawaraka() {
        let ctx = ctx();
        let kf = kf_sa();
        let result = suffix_sa(&ctx, "やわらか", "さ", &kf).unwrap();
        assert_eq!(result.len(), 2);
        for c in &result {
            assert_eq!(c.text, "やわらかさ");
            assert_eq!(c.kana, "やわらかさ");
            assert!(matches!(c.score_mod, ScoreMod::Single(2)));
            assert!(c.score_base.is_none());
            match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => assert_eq!(k.text, "やわらか"),
                other => panic!("expected Kana primary, got {:?}", other),
            }
            assert_eq!(c.words.len(), 2);
            match &c.words[1] {
                KaniWordDispatchEnum::Kana(k) => {
                    assert_eq!(k.seq, kf.seq);
                    assert_eq!(k.text, kf.text);
                }
                other => panic!("expected Kana word2 (kf), got {:?}", other),
            }
        }
        // nconc order: arm-A (conj-type 51) first, arm-B (adj-na) second.
        let ids: Vec<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.id,
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(ids, vec![1018986, 53460]);
        let seqs: Vec<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(seqs, vec![10639355, 1460730]);
    }

    /// REPL SA4: `(suffix-sa "食べる" "さ" kf-sa)` → NIL. 食べる is a
    /// verb, neither an adj-i stem (conj-type 51) nor an adj-na noun.
    #[test]
    fn sa4_no_match_verb() {
        let ctx = ctx();
        let kf = kf_sa();
        let result = suffix_sa(&ctx, "食べる", "さ", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL SA5: `(suffix-sa "高" "さ" kf-sa)` → 1 COMPOUND
    /// text="高さ" kana="たかさ" score-mod=2 score-base=NIL
    /// primary=KANJI-TEXT (高 id=1422119 seq=10591797),
    /// words=(primary, kf-sa). Exercises arm A on a single-char kanji
    /// stem.
    #[test]
    fn sa5_adj_i_stem_single_kanji() {
        let ctx = ctx();
        let kf = kf_sa();
        let result = suffix_sa(&ctx, "高", "さ", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "高さ");
        assert_eq!(c.kana, "たかさ");
        assert!(matches!(c.score_mod, ScoreMod::Single(2)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 1422119);
                assert_eq!(k.seq, 10591797);
                assert_eq!(k.text, "高");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }
}

mod suffix_iadj {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:iadj` suffix-cache `kf` for "げ", REPL pinned: `(get-kana-form
    /// 2006580 "げ")` → id=107976, seq=2006580, text="げ", common=:NULL,
    /// common_tags="", conjugate_p=T, nokanji=nil, best_kanji="気".
    fn kf_iadj_ge() -> KanaText {
        KanaText {
            id: 107976,
            seq: 2006580,
            text: "げ".into(),
            ord: 0,
            common: None,
            common_tags: String::new().into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: Some("気".into()),
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL IADJ1: `(suffix-iadj "悲し" "げ" kf-iadj-ge)` → 1
    /// COMPOUND text="悲しげ" kana="かなしげ" score-mod=1
    /// score-base=NIL primary=KANJI-TEXT (悲し seq 10101813),
    /// words=(primary kf).
    #[test]
    fn iadj1_kanashi_ge() {
        let ctx = ctx();
        let kf = kf_iadj_ge();
        let result = suffix_iadj(&ctx, "悲し", "げ", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "悲しげ");
        assert_eq!(c.kana, "かなしげ");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "悲し");
                assert_eq!(k.seq, 10101813);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL IADJ2: `(suffix-iadj "嬉し" "げ" kf-iadj-ge)` → 1
    /// COMPOUND text="嬉しげ" kana="うれしげ" primary=KANJI-TEXT
    /// (嬉し seq 10215030).
    #[test]
    fn iadj2_ureshi_ge() {
        let ctx = ctx();
        let kf = kf_iadj_ge();
        let result = suffix_iadj(&ctx, "嬉し", "げ", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "嬉しげ");
        assert_eq!(c.kana, "うれしげ");
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "嬉し");
                assert_eq!(k.seq, 10215030);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL IADJ3: `(suffix-iadj "やわらか" "げ" kf-iadj-ge)` → 1
    /// COMPOUND text="やわらかげ" kana="やわらかげ" primary=KANA-TEXT
    /// (やわらか seq 10639355). Exercises the kana-text arm.
    #[test]
    fn iadj3_yawaraka_ge_kana() {
        let ctx = ctx();
        let kf = kf_iadj_ge();
        let result = suffix_iadj(&ctx, "やわらか", "げ", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "やわらかげ");
        assert_eq!(c.kana, "やわらかげ");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "やわらか");
                assert_eq!(k.seq, 10639355);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }

    /// REPL IADJ4: `(suffix-iadj "無理" "げ" kf-iadj-ge)` → NIL.
    /// 無理 has no conj-type-51 row.
    #[test]
    fn iadj4_non_adjective_root() {
        let ctx = ctx();
        let kf = kf_iadj_ge();
        let result = suffix_iadj(&ctx, "無理", "げ", &kf).unwrap();
        assert!(result.is_empty());
    }
}

mod suffix_garu {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:garu` cache `kf` for "がる", REPL pinned: id=72111, seq=1631750,
    /// text="がる", ord=0, common=:NULL, common_tags="[spec1]",
    /// conjugate_p=T, nokanji=NIL, best_kanji=:NULL.
    fn kf_garu() -> KanaText {
        KanaText {
            id: 72111,
            seq: 1631750,
            text: "がる".into(),
            ord: 0,
            common: None,
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL: `(suffix-garu "寒" "がる" kf)` → 1 COMPOUND text="寒がる"
    /// kana="さむがる" score-mod=0 primary=KANJI-TEXT (寒 id=148342 seq=2453760).
    /// Hits the conj-adj-stem arm with a kanji root.
    #[test]
    fn garu1_adj_stem_kanji() {
        let ctx = ctx();
        let kf = kf_garu();
        let result = suffix_garu(&ctx, "寒", "がる", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "寒がる");
        assert_eq!(c.kana, "さむがる");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 148342);
                assert_eq!(k.seq, 2453760);
                assert_eq!(k.text, "寒");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-garu "怖" "がる" kf)` → 1 COMPOUND text="怖がる"
    /// kana="こわがる" primary=KANJI-TEXT (怖 seq 2259840).
    #[test]
    fn garu2_kowa() {
        let ctx = ctx();
        let kf = kf_garu();
        let result = suffix_garu(&ctx, "怖", "がる", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "怖がる");
        assert_eq!(c.kana, "こわがる");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 2259840),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-garu "欲し" "がる" kf)` → 1 COMPOUND text="欲しがる"
    /// kana="ほしがる" primary=KANJI-TEXT (欲し seq 10139646). Pins
    /// adj-stem on a 2-char kanji root.
    #[test]
    fn garu3_hoshi() {
        let ctx = ctx();
        let kf = kf_garu();
        let result = suffix_garu(&ctx, "欲し", "がる", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "欲しがる");
        assert_eq!(c.kana, "ほしがる");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10139646),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-garu "広" "がる" kf)` → 1 COMPOUND text="広がる"
    /// kana="ひろがる" primary=KANJI-TEXT (広 seq 10420123).
    #[test]
    fn garu4_hiro() {
        let ctx = ctx();
        let kf = kf_garu();
        let result = suffix_garu(&ctx, "広", "がる", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "広がる");
        assert_eq!(c.kana, "ひろがる");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10420123),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: each of `"な" "い" "よ"` → NIL via the outer
    /// `(unless (member root …))` guard.
    #[test]
    fn garu5_member_excludes() {
        let ctx = ctx();
        let kf = kf_garu();
        for r in ["な", "い", "よ"] {
            let result = suffix_garu(&ctx, r, "がる", &kf).unwrap();
            assert!(result.is_empty(), "expected NIL for root={:?}", r);
        }
    }

    /// REPL: `(suffix-garu "食べた" "がる" kf)` and `"行きた"` → NIL.
    /// "食べた" / "行きた" are conj-type-2 (past) stems, not adj-stems
    /// (conj-type 51), and don't end with "そ"; both arms yield NIL.
    #[test]
    fn garu6_tai_stem_no_match() {
        let ctx = ctx();
        let kf = kf_garu();
        for r in ["食べた", "行きた"] {
            let result = suffix_garu(&ctx, r, "がる", &kf).unwrap();
            assert!(result.is_empty(), "expected NIL for root={:?}", r);
        }
    }

    /// REPL: `(suffix-garu "行きそ" "がる" kf)` → 1 COMPOUND text="行きそがる"
    /// kana="いきそがる" score-mod=(0 (constantly N)) primary=KANJI-TEXT
    /// (行き seq 10349442) nwords=3. Exercises the `(ends-with "そ" root)`
    /// patch branch: patch=("う",""), new-root="行きそう",
    /// find-word-with-suffix on `:sou` returns a compound (行き+そう);
    /// adjoin-word wraps that compound with kf-garu, building text from
    /// the outer root ("行きそ"+"がる") and kana via
    /// `destem(compound-kana, length("う")=1) + "" + "" + suf` =
    /// destem("いきそう",1)+"がる"="いきそ"+"がる". Score-mod stacks the
    /// inner suffix-sou's constantly behind the integer 0.
    #[test]
    fn garu7_so_patch_branch_kanji() {
        let ctx = ctx();
        let kf = kf_garu();
        let result = suffix_garu(&ctx, "行きそ", "がる", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "行きそがる");
        assert_eq!(c.kana, "いきそがる");
        // dict.lisp:651 — :score-mod stacks (list new old) when the
        // pre-existing slot was a non-list closure.
        match &c.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 2);
                assert!(matches!(v[0], ScoreMod::Single(0)));
                assert!(matches!(v[1], ScoreMod::Constant(_)));
            }
            other => panic!("expected Stack score_mod, got {:?}", other),
        }
        assert_eq!(c.words.len(), 3);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10349442),
            other => panic!(
                "expected Kanji primary (inner suffix-sou primary), got {:?}",
                other
            ),
        }
    }

    /// REPL: `(suffix-garu "そ" "がる" kf)` → NIL. Arm A: conj-type-51 on
    /// "そ" → 0. Arm B (so-tail): new-root="そう"; find-word-with-suffix
    /// "そう" :sou → 0 because the cache has no compound suffix-class
    /// :sou entry for "そう".
    #[test]
    fn garu8_so_only_no_match() {
        let ctx = ctx();
        let kf = kf_garu();
        let result = suffix_garu(&ctx, "そ", "がる", &kf).unwrap();
        assert!(result.is_empty());
    }
}

mod suffix_ra {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::dao::SimpleText;

    /// `:ra` suffix-cache `kf`, REPL pinned: `(get-kana-form 2067770
    /// "ら")` → id=114553, seq=2067770, text="ら", common=:NULL,
    /// common_tags="[spec1]", conjugate_p=T, nokanji=nil,
    /// best_kanji="等", hintedp=nil. Pulled verbatim from
    /// `corpus/extracted_chunk_c_suffix_abbr_2026_05_16/dict/\
    /// suffix_ra.parquet` row 3750.
    fn kf_ra() -> KanaText {
        KanaText {
            id: 114553,
            seq: 2067770,
            text: "ら".into(),
            ord: 0,
            common: None,
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: Some("等".into()),
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL RA1: `(suffix-ra "我々" "ら" kf-ra)` → 1 COMPOUND
    /// text="我々ら" kana="われわれら" score-mod=1 primary=KANJI-TEXT
    /// (我々 seq 1607050).
    #[test]
    fn ra1_kanji_pn_match() {
        let ctx = ctx();
        let kf = kf_ra();
        let result = suffix_ra(&ctx, "我々", "ら", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "我々ら");
        assert_eq!(c.kana, "われわれら");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "我々");
                assert_eq!(k.seq, 1607050);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
    }

    /// REPL RA2: `(suffix-ra "ばら" "ら" kf-ra)` → 0 (the UNLESS
    /// branch fires for any root ending in ら).
    #[test]
    fn ra2_root_ends_with_ra() {
        let ctx = ctx();
        let kf = kf_ra();
        let result = suffix_ra(&ctx, "ばら", "ら", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL RA3: `(suffix-ra "私" "ら" kf-ra)` → 13 COMPOUNDs (one
    /// per kanji-text row of 私 with a `pn` sense). Exercises the
    /// polysemy + multi-kana branch (`get-kana` returns a different
    /// best_kana per row → distinct compound `kana` values).
    #[test]
    fn ra3_kanji_pn_polysemy_thirteen_compounds() {
        let ctx = ctx();
        let kf = kf_ra();
        let result = suffix_ra(&ctx, "私", "ら", &kf).unwrap();
        assert_eq!(result.len(), 13);
        for c in &result {
            assert_eq!(c.text, "私ら");
            assert!(matches!(c.score_mod, ScoreMod::Single(1)));
            assert!(c.score_base.is_none());
            match &*c.primary {
                KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.text, "私"),
                other => panic!("expected Kanji primary, got {:?}", other),
            }
            assert_eq!(c.words.len(), 2);
            match &c.words[1] {
                KaniWordDispatchEnum::Kana(k) => {
                    assert_eq!(k.seq, kf.seq);
                    assert_eq!(k.text, kf.text);
                }
                other => panic!("expected Kana word2 (kf), got {:?}", other),
            }
        }
        let mut got: Vec<String> = result.iter().map(|c| c.kana.clone()).collect();
        got.sort();
        let mut expected: Vec<String> = vec![
            "わたしら",
            "あたしら",
            "わらわら",
            "わしら",
            "あっしら",
            "わいら",
            "わたいら",
            "わたくしら",
            "わっちら",
            "わてら",
            "あたいら",
            "あてら",
            "わちきら",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        expected.sort();
        assert_eq!(got, expected);
    }

    /// REPL RA4: `(suffix-ra "青空" "ら" kf-ra)` → 0 (青空 has no
    /// `pn` sense and 1580640 is the seq of 等, not 青空).
    #[test]
    fn ra4_kanji_no_pn_no_fallback_seq() {
        let ctx = ctx();
        let kf = kf_ra();
        let result = suffix_ra(&ctx, "青空", "ら", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL RA5: `(suffix-ra "等" "ら" kf-ra)` → 0. The seq 1580640
    /// is reserved for the find-word-seq fallback path; 等 does not
    /// hit `pn` so `or-as-hiragana` falls through, and 等 itself isn't
    /// a kanji_text row at that seq, so find-word-seq also misses.
    #[test]
    fn ra5_etc_kanji_no_match() {
        let ctx = ctx();
        let kf = kf_ra();
        let result = suffix_ra(&ctx, "等", "ら", &kf).unwrap();
        assert!(result.is_empty());
    }

    /// REPL RA6: `(suffix-ra "アナタ" "ら" kf-ra)` → 2 COMPOUNDs
    /// where each primary is a PROXY-TEXT wrapping the hiragana
    /// kana_text あなた (`or-as-hiragana` fallback path). The
    /// compound's `kana` is built off the proxy's `kana` slot, which
    /// carries the katakana surface form — so `kana = "アナタら"`.
    #[test]
    fn ra6_katakana_as_hiragana_proxy_fallback() {
        let ctx = ctx();
        let kf = kf_ra();
        let result = suffix_ra(&ctx, "アナタ", "ら", &kf).unwrap();
        assert_eq!(result.len(), 2);
        for c in &result {
            assert_eq!(c.text, "アナタら");
            assert_eq!(c.kana, "アナタら");
            assert!(matches!(c.score_mod, ScoreMod::Single(1)));
            assert!(c.score_base.is_none());
            match &*c.primary {
                KaniWordDispatchEnum::Proxy(p) => assert_eq!(p.text, "アナタ"),
                other => panic!("expected Proxy primary, got {:?}", other),
            }
            assert_eq!(c.words.len(), 2);
            match &c.words[1] {
                KaniWordDispatchEnum::Kana(k) => {
                    assert_eq!(k.seq, kf.seq);
                    assert_eq!(k.text, kf.text);
                }
                other => panic!("expected Kana word2 (kf), got {:?}", other),
            }
        }
    }

    /// REPL RA7: `(suffix-ra "わたし" "ら" kf-ra)` → 1 COMPOUND
    /// text="わたしら" kana="わたしら" score-mod=1 score-base=NIL
    /// primary=KANA-TEXT (わたし seq 1311110), words=(primary kf).
    /// Exercises the hiragana-direct `kana_text` arm of
    /// `find-word-with-pos`.
    #[test]
    fn ra7_hiragana_direct_kana_text() {
        let ctx = ctx();
        let kf = kf_ra();
        let result = suffix_ra(&ctx, "わたし", "ら", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "わたしら");
        assert_eq!(c.kana, "わたしら");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "わたし");
                assert_eq!(k.seq, 1311110);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }
}

mod suffix_rashii {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:rashii` cache `kf` for "らしい", REPL pinned: id=1812,
    /// seq=1013240, text="らしい", ord=0, common=0,
    /// common_tags="[ichi1]", conjugate_p=T, nokanji=NIL,
    /// best_kanji=:NULL.
    fn kf_rashii() -> KanaText {
        KanaText {
            id: 1812,
            seq: 1013240,
            text: "らしい".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[ichi1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL: `(suffix-rashii "食べた" "らしい" kf)` → 1 COMPOUND
    /// text="食べたらしい" kana="たべたらしい" score-mod=3 primary=KANJI-TEXT
    /// (食べた id=411235 seq=10092229), score-base=KANJI-TEXT
    /// (食べたら id=411321 seq=10092265). pair-words-by-conj paired
    /// 食べた (conj-type 2) with 食べたら (conj-type 11) by shared
    /// (seq-from, via) signature.
    #[test]
    fn rashii1_tabeta() {
        let ctx = ctx();
        let kf = kf_rashii();
        let result = suffix_rashii(&ctx, "食べた", "らしい", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べたらしい");
        assert_eq!(c.kana, "たべたらしい");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 411235);
                assert_eq!(k.seq, 10092229);
                assert_eq!(k.text, "食べた");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        let score_base = c.score_base.as_ref().expect("score-base must be set");
        match score_base.as_ref() {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 411321);
                assert_eq!(k.seq, 10092265);
                assert_eq!(k.text, "食べたら");
            }
            other => panic!("expected Kanji score-base (食べたら), got {:?}", other),
        }
    }

    /// REPL: `(suffix-rashii "来た" "らしい" kf)` → 1 COMPOUND
    /// text="来たらしい" kana="きたらしい" primary=KANJI-TEXT (来た
    /// id=670727 seq=10221106), score-base=KANJI-TEXT (来たら id=670813
    /// seq=10221142).
    #[test]
    fn rashii2_kita() {
        let ctx = ctx();
        let kf = kf_rashii();
        let result = suffix_rashii(&ctx, "来た", "らしい", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "来たらしい");
        assert_eq!(c.kana, "きたらしい");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10221106),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        let sb = c.score_base.as_ref().expect("score-base set");
        match sb.as_ref() {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10221142),
            other => panic!("expected Kanji score-base (来たら), got {:?}", other),
        }
    }

    /// REPL: `(suffix-rashii "行った" "らしい" kf)` → 3 COMPOUNDs each
    /// text="行ったらしい". Per-bucket REPL pairings of (primary_seq,
    /// score_base_seq):
    ///   - (10402883, 10402923) — やった / やったら
    ///   - (10349394, 10349434) — いった / いったら
    ///   - (10087633, 10087672) — おこなった / おこなったら
    /// Pin each pair so pair-words-by-conj keeps its bucket signature.
    #[test]
    fn rashii3_itta_three_readings() {
        let ctx = ctx();
        let kf = kf_rashii();
        let result = suffix_rashii(&ctx, "行った", "らしい", &kf).unwrap();
        assert_eq!(result.len(), 3);
        let expected_pairs: std::collections::HashMap<i32, i32> = [
            (10402883, 10402923),
            (10349394, 10349434),
            (10087633, 10087672),
        ]
        .into_iter()
        .collect();
        for c in &result {
            assert_eq!(c.text, "行ったらしい");
            assert!(matches!(c.score_mod, ScoreMod::Single(3)));
            assert_eq!(c.words.len(), 2);
            let primary_seq = match &*c.primary {
                KaniWordDispatchEnum::Kanji(k) => k.seq,
                other => panic!("expected Kanji primary, got {:?}", other),
            };
            let expected_score_base_seq = expected_pairs
                .get(&primary_seq)
                .unwrap_or_else(|| panic!("unexpected primary seq {}", primary_seq));
            let sb = c.score_base.as_ref().expect("score-base set");
            match sb.as_ref() {
                KaniWordDispatchEnum::Kanji(k) => assert_eq!(
                    k.seq, *expected_score_base_seq,
                    "primary {} should pair with score-base {}",
                    primary_seq, expected_score_base_seq
                ),
                other => panic!("expected Kanji score-base, got {:?}", other),
            }
        }
    }

    /// REPL: `(suffix-rashii "した" "らしい" kf)` → 1 COMPOUND
    /// text="したらしい" kana="したらしい" primary=KANA-TEXT (した
    /// id=439677 seq=10152246), score-base=KANA-TEXT (したら id=439719
    /// seq=10152284). Exercises kana-text dispatch.
    #[test]
    fn rashii4_shita_kana() {
        let ctx = ctx();
        let kf = kf_rashii();
        let result = suffix_rashii(&ctx, "した", "らしい", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "したらしい");
        assert_eq!(c.kana, "したらしい");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => assert_eq!(k.seq, 10152246),
            other => panic!("expected Kana primary, got {:?}", other),
        }
        let sb = c.score_base.as_ref().expect("score-base set");
        match sb.as_ref() {
            KaniWordDispatchEnum::Kana(k) => assert_eq!(k.seq, 10152284),
            other => panic!("expected Kana score-base (したら), got {:?}", other),
        }
    }

    /// REPL: each of `"無理" "食べ"` → NIL. "無理" has no conj-type 2 row
    /// and "無理ら" has no conj-type 11; "食べ" is conj-type 13 (ren-stem)
    /// not 2, and "食べら" has no conj-type 11.
    #[test]
    fn rashii5_no_conjugation_pair() {
        let ctx = ctx();
        let kf = kf_rashii();
        for r in ["無理", "食べ"] {
            let result = suffix_rashii(&ctx, r, "らしい", &kf).unwrap();
            assert!(result.is_empty(), "expected NIL for root={:?}", r);
        }
    }
}

mod suffix_desu {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:desu` cache `kf` for "です", REPL pinned: id=71736, seq=1628500,
    /// text="です", ord=0, common=0, common_tags="[spec1]",
    /// conjugate_p=T, nokanji=NIL, best_kanji=:NULL.
    fn kf_desu() -> KanaText {
        KanaText {
            id: 71736,
            seq: 1628500,
            text: "です".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL: `(suffix-desu "食べない" "です" kf)` → 1 COMPOUND
    /// text="食べないです" kana="たべない です" score-mod=(constantly 200)
    /// connector=" " primary=KANJI-TEXT (食べない id=411231 seq=10092227).
    /// "ない"-tail branch into conj-neg filter.
    #[test]
    fn desu1_nai_tail_kanji() {
        let ctx = ctx();
        let kf = kf_desu();
        let result = suffix_desu(&ctx, "食べない", "です", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べないです");
        assert_eq!(c.kana, "たべない です");
        assert!(matches!(c.score_mod, ScoreMod::Constant(200)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 411231);
                assert_eq!(k.seq, 10092227);
                assert_eq!(k.text, "食べない");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-desu "ない" "です" kf)` → 3 COMPOUNDs text="ないです"
    /// kana="ない です" each — find-word-with-conj-prop on bare "ない"
    /// yields three kana-text rows (seqs 2257550 / 10320151 / 10470712).
    #[test]
    fn desu2_bare_nai_kana() {
        let ctx = ctx();
        let kf = kf_desu();
        let result = suffix_desu(&ctx, "ない", "です", &kf).unwrap();
        assert_eq!(result.len(), 3);
        for c in &result {
            assert_eq!(c.text, "ないです");
            assert_eq!(c.kana, "ない です");
            assert!(matches!(c.score_mod, ScoreMod::Constant(200)));
            assert!(c.score_base.is_none());
            assert_eq!(c.words.len(), 2);
        }
        let seqs: std::collections::HashSet<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => -1,
            })
            .collect();
        assert!(seqs.contains(&2257550));
        assert!(seqs.contains(&10320151));
        assert!(seqs.contains(&10470712));
    }

    /// REPL: `(suffix-desu "行かなかった" "です" kf)` → 1 COMPOUND
    /// text="行かなかったです" kana="いかなかった です"
    /// primary=KANJI-TEXT (行かなかった id=922673 seq=10349396). Exercises
    /// the "なかった"-tail branch.
    #[test]
    fn desu3_nakatta_tail() {
        let ctx = ctx();
        let kf = kf_desu();
        let result = suffix_desu(&ctx, "行かなかった", "です", &kf)
            
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "行かなかったです");
        assert_eq!(c.kana, "いかなかった です");
        assert!(matches!(c.score_mod, ScoreMod::Constant(200)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 922673);
                assert_eq!(k.seq, 10349396);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-desu "じゃない" "です" kf)` → 1 COMPOUND
    /// text="じゃないです" kana="じゃない です" primary=KANA-TEXT
    /// (じゃない id=3289329 seq=10019714 ord=1). Pins a じゃない-tail
    /// case that still goes through the "ない" check.
    #[test]
    fn desu4_janai_tail() {
        let ctx = ctx();
        let kf = kf_desu();
        let result = suffix_desu(&ctx, "じゃない", "です", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "じゃないです");
        assert_eq!(c.kana, "じゃない です");
        assert!(matches!(c.score_mod, ScoreMod::Constant(200)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.id, 3289329);
                assert_eq!(k.seq, 10019714);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }

    /// REPL: each of `"食べ"`, `"行きません"`, `"ありません"` → NIL —
    /// none end with "ない" / "なかった", so the outer `and` short-circuits.
    #[test]
    fn desu5_no_nai_suffix_returns_empty() {
        let ctx = ctx();
        let kf = kf_desu();
        for r in ["食べ", "行きません", "ありません"] {
            let result = suffix_desu(&ctx, r, "です", &kf).unwrap();
            assert!(result.is_empty(), "expected NIL for root={:?}", r);
        }
    }
}

mod suffix_desho {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:desho` cache `kf` for "でしょう", REPL pinned: id=1122,
    /// seq=1008420, text="でしょう", ord=0, common=0,
    /// common_tags="[spec1]", conjugate_p=T, nokanji=NIL,
    /// best_kanji=:NULL. The `:desho` key also has a "でしょ" cache row
    /// (id=1123, ord=1) loaded by `(load-kf :desho (get-kana-form 1008420
    /// "でしょ"))` at `dict-grammar.lisp:271` — exercised by `desho4`.
    fn kf_deshou() -> KanaText {
        KanaText {
            id: 1122,
            seq: 1008420,
            text: "でしょう".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn kf_desho_short() -> KanaText {
        KanaText {
            id: 1123,
            seq: 1008420,
            text: "でしょ".into(),
            ord: 1,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL: `(suffix-desho "食べない" "でしょう" kf)` → 1 COMPOUND
    /// text="食べないでしょう" kana="たべない でしょう"
    /// score-mod=(constantly 300) connector=" " primary=KANJI-TEXT
    /// (食べない seq 10092227).
    #[test]
    fn desho1_nai_tail_kanji() {
        let ctx = ctx();
        let kf = kf_deshou();
        let result = suffix_desho(&ctx, "食べない", "でしょう", &kf)
            
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べないでしょう");
        assert_eq!(c.kana, "たべない でしょう");
        assert!(matches!(c.score_mod, ScoreMod::Constant(300)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10092227),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-desho "ない" "でしょう" kf)` → 3 COMPOUNDs
    /// text="ないでしょう" each. Same 3 ない seqs as the desu test.
    #[test]
    fn desho2_bare_nai() {
        let ctx = ctx();
        let kf = kf_deshou();
        let result = suffix_desho(&ctx, "ない", "でしょう", &kf).unwrap();
        assert_eq!(result.len(), 3);
        for c in &result {
            assert_eq!(c.text, "ないでしょう");
            assert_eq!(c.kana, "ない でしょう");
            assert!(matches!(c.score_mod, ScoreMod::Constant(300)));
            assert!(c.score_base.is_none());
            assert_eq!(c.words.len(), 2);
        }
    }

    /// REPL: `(suffix-desho "行かない" "でしょう" kf)` → 1 COMPOUND
    /// text="行かないでしょう" kana="いかない でしょう" primary=KANJI-TEXT
    /// (行かない id=922665 seq=10349392).
    #[test]
    fn desho3_ikanai() {
        let ctx = ctx();
        let kf = kf_deshou();
        let result = suffix_desho(&ctx, "行かない", "でしょう", &kf)
            
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "行かないでしょう");
        assert_eq!(c.kana, "いかない でしょう");
        assert!(matches!(c.score_mod, ScoreMod::Constant(300)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 922665);
                assert_eq!(k.seq, 10349392);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-desho "ない" "でしょ" kf-short)` → 3 COMPOUNDs
    /// text="ないでしょ" each, kana="ない でしょ". Exercises the short
    /// "でしょ" `kf` (cache id=1123).
    #[test]
    fn desho4_short_desho_kf() {
        let ctx = ctx();
        let kf = kf_desho_short();
        let result = suffix_desho(&ctx, "ない", "でしょ", &kf).unwrap();
        assert_eq!(result.len(), 3);
        for c in &result {
            assert_eq!(c.text, "ないでしょ");
            assert_eq!(c.kana, "ない でしょ");
            assert!(matches!(c.score_mod, ScoreMod::Constant(300)));
            assert!(c.score_base.is_none());
            assert_eq!(c.words.len(), 2);
        }
    }

    /// REPL: each of `"食べ"`, `"ありません"`, `"行かなかった"` → NIL.
    /// Unlike suffix-desu, suffix-desho only takes "ない" tails, so
    /// "なかった" tails fall through.
    #[test]
    fn desho5_no_nai_tail_returns_empty() {
        let ctx = ctx();
        let kf = kf_deshou();
        for r in ["食べ", "ありません", "行かなかった"] {
            let result = suffix_desho(&ctx, r, "でしょう", &kf).unwrap();
            assert!(result.is_empty(), "expected NIL for root={:?}", r);
        }
    }
}

mod suffix_tosuru {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:tosuru` suffix-cache `kf` for "とする" — the root kana-text of
    /// seq 2136890. REPL pinned: id=122279, seq=2136890, text="とする",
    /// common=:NULL, common_tags="[spec1]", conjugate_p=T, nokanji=nil,
    /// best_kanji=:NULL. The `(load-conjs :tosuru 2136890)` loader walks
    /// every kana-form of 2136890; we pick the root row.
    fn kf_tosuru() -> KanaText {
        KanaText {
            id: 122279,
            seq: 2136890,
            text: "とする".into(),
            ord: 0,
            common: None,
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL TOSURU1: `(suffix-tosuru "食べよう" "とする" kf-tosuru)` → 1
    /// COMPOUND text="食べようとする" kana="たべよう とする"
    /// score-mod=3 score-base=NIL primary=KANJI-TEXT (食べよう seq
    /// 10092257), words=(primary kf). Note the space in kana from
    /// connector=" ".
    #[test]
    fn tosuru1_taberu_volitional_kanji() {
        let ctx = ctx();
        let kf = kf_tosuru();
        let result = suffix_tosuru(&ctx, "食べよう", "とする", &kf)
            
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べようとする");
        assert_eq!(c.kana, "たべよう とする");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べよう");
                assert_eq!(k.seq, 10092257);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL TOSURU2: `(suffix-tosuru "行こう" "とする" kf-tosuru)` → 1
    /// COMPOUND text="行こうとする" kana="いこう とする" score-mod=3
    /// primary=KANJI-TEXT (行こう seq 10349426).
    #[test]
    fn tosuru2_ikou() {
        let ctx = ctx();
        let kf = kf_tosuru();
        let result = suffix_tosuru(&ctx, "行こう", "とする", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "行こうとする");
        assert_eq!(c.kana, "いこう とする");
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "行こう");
                assert_eq!(k.seq, 10349426);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL TOSURU3: `(suffix-tosuru "なろう" "とする" kf-tosuru)` → 3
    /// COMPOUNDs (KANA-TEXT polysemy of なろう as volitional).
    /// Each compound has text="なろうとする" kana="なろう とする"
    /// score-mod=3 primary=KANA-TEXT (なろう). Pinned seqs:
    /// 10052616, 10374864, 10549414.
    #[test]
    fn tosuru3_narou_polysemy_three() {
        let ctx = ctx();
        let kf = kf_tosuru();
        let result = suffix_tosuru(&ctx, "なろう", "とする", &kf).unwrap();
        assert_eq!(result.len(), 3);
        for c in &result {
            assert_eq!(c.text, "なろうとする");
            assert_eq!(c.kana, "なろう とする");
            assert!(matches!(c.score_mod, ScoreMod::Single(3)));
            match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => assert_eq!(k.text, "なろう"),
                other => panic!("expected Kana primary, got {:?}", other),
            }
        }
        let mut seqs: Vec<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => unreachable!(),
            })
            .collect();
        seqs.sort();
        assert_eq!(seqs, vec![10052616, 10374864, 10549414]);
    }

    /// REPL TOSURU4: `(suffix-tosuru "無理" "とする" kf-tosuru)` → NIL.
    /// 無理 has no conj-type-9 (volitional) row.
    #[test]
    fn tosuru4_non_verb_root() {
        let ctx = ctx();
        let kf = kf_tosuru();
        let result = suffix_tosuru(&ctx, "無理", "とする", &kf).unwrap();
        assert!(result.is_empty());
    }
}

mod suffix_kurai {
    use crate::dict::grammar::suffix::rules::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::dao::SimpleText;

    /// `:kurai` suffix-cache `kf` for "くらい", REPL pinned: `(get-kana-
    /// form 1154340 "くらい")` → id=21985, seq=1154340, text="くらい",
    /// common=0, common_tags="[spec1]", conjugate_p=T, nokanji=nil,
    /// best_kanji=:NULL.
    fn kf_kurai() -> KanaText {
        KanaText {
            id: 21985,
            seq: 1154340,
            text: "くらい".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL KURAI1: `(suffix-kurai "食べた" "くらい" kf-kurai)` → 1
    /// COMPOUND text="食べたくらい" kana="たべた くらい" score-mod=3
    /// score-base=NIL primary=KANJI-TEXT (食べた seq 10092229),
    /// words=(primary kf). Note the space in kana from connector=" ".
    #[test]
    fn kurai1_tabeta_kurai_kanji() {
        let ctx = ctx();
        let kf = kf_kurai();
        let result = suffix_kurai(&ctx, "食べた", "くらい", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べたくらい");
        assert_eq!(c.kana, "たべた くらい");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べた");
                assert_eq!(k.seq, 10092229);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL KURAI2: `(suffix-kurai "見た" "くらい" kf-kurai)` → 1
    /// COMPOUND text="見たくらい" kana="みた くらい" score-mod=3
    /// primary=KANJI-TEXT (見た seq 10315009).
    #[test]
    fn kurai2_mita_kurai() {
        let ctx = ctx();
        let kf = kf_kurai();
        let result = suffix_kurai(&ctx, "見た", "くらい", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "見たくらい");
        assert_eq!(c.kana, "みた くらい");
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "見た");
                assert_eq!(k.seq, 10315009);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL KURAI3: `(suffix-kurai "した" "くらい" kf-kurai)` → 1
    /// COMPOUND text="したくらい" kana="した くらい" primary=KANA-TEXT
    /// (した seq 10152246). Exercises the kana-text arm.
    #[test]
    fn kurai3_shita_kana() {
        let ctx = ctx();
        let kf = kf_kurai();
        let result = suffix_kurai(&ctx, "した", "くらい", &kf).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "したくらい");
        assert_eq!(c.kana, "した くらい");
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "した");
                assert_eq!(k.seq, 10152246);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }

    /// REPL KURAI4: `(suffix-kurai "無理" "くらい" kf-kurai)` → NIL.
    /// 無理 has no conj-type-2 row.
    #[test]
    fn kurai4_non_verb_root() {
        let ctx = ctx();
        let kf = kf_kurai();
        let result = suffix_kurai(&ctx, "無理", "くらい", &kf).unwrap();
        assert!(result.is_empty());
    }
}
