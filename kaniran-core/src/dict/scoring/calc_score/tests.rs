mod calc_score {
    use crate::dict::dao::KanaText;
    use crate::dict::dao::SimpleText;
    use crate::dict::readings::{find_word, FindWordRows};
    use crate::dict::scoring::calc_score::*;
    use crate::dict::scoring::score::KaniSplitInfo;
    use crate::dict::text_classes::CompoundText;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn first_word_for(ctx: &KaniranContext, s: &str) -> KaniWordDispatchEnum {
        match find_word(ctx, s, false).await.unwrap() {
            FindWordRows::Kana(mut v) => KaniWordDispatchEnum::Kana(v.remove(0)),
            FindWordRows::Kanji(mut v) => KaniWordDispatchEnum::Kanji(v.remove(0)),
        }
    }

    /// Fetch a specific kana_text row by (seq, text) — deterministic
    /// alternative to `first_word_for` when find-word's row order
    /// (no upstream ORDER BY) would make a test flaky.
    async fn kana_by_seq_text(ctx: &KaniranContext, seq: i32, text: &str) -> KaniWordDispatchEnum {
        let rows: Vec<crate::dict::dao::KanaText> = sqlx::query_as(
            "SELECT * FROM kana_text WHERE seq = $1 AND text = $2 ORDER BY id LIMIT 1",
        )
        .bind(seq)
        .bind(text)
        .fetch_all(&ctx.pool)
        .await
        .expect("query");
        KaniWordDispatchEnum::Kana(rows.into_iter().next().expect("row exists"))
    }

    // ----- REPL-pinned cases (.103, 2026-05-16). All output strings
    //       captured by `(calc-score (car (find-word "<txt>")) …)`. -----

    /// REPL: `(calc-score (car (find-word "ねこ")))` →
    ///   `16  (:POSI ("n") :SEQ-SET (1467640) :CONJ NIL :COMMON 7 :SCORE-INFO (4 NIL 0 NIL) :KPCL (NIL NIL T NIL))`
    #[tokio::test]
    async fn nekko_baseline() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "ねこ").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
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

    /// REPL: `(calc-score (car (find-word "の")))` →
    ///   `11  (:POSI ("prt") :SEQ-SET (1469800) :CONJ NIL :COMMON 0 :SCORE-INFO (11 NIL 0 NIL) :KPCL (NIL T T NIL))`
    #[tokio::test]
    async fn no_particle_non_final() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "の").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
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

    /// REPL: `(calc-score (car (find-word "の")) :final t)` →
    ///   `16  (:POSI ("prt") :SEQ-SET (1469800) :CONJ NIL :COMMON 0 :SCORE-INFO (16 NIL 0 NIL) :KPCL (NIL T T NIL))`
    #[tokio::test]
    async fn no_particle_final_branch_bonus() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "の").await;
        let (score, info) = calc_score(&ctx, &w, true, None, None, &[]).await.unwrap();
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

    /// REPL: `(calc-score (car (find-word "は")))` →
    ///   `1  (:POSI ("n") :SEQ-SET (1171680) :CONJ NIL :COMMON NIL :SCORE-INFO (1 NIL 0 NIL) :KPCL (NIL NIL NIL NIL))`
    #[tokio::test]
    async fn ha_n_uncommon() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "は").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
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

    /// REPL: with row `(select-dao 'kana-text (:and (:= 'seq 2089020) (:= 'text "だ")))` →
    ///   `(calc-score row)` and `(calc-score row :final t)` both →
    ///   `16  (:POSI ("aux-v" "cop" "cop-da") :SEQ-SET (2089020) :CONJ NIL :COMMON 0
    ///         :SCORE-INFO (16 NIL 0 NIL) :KPCL (NIL T T NIL))`
    ///
    /// Pinned via the explicit-seq helper because `(find-word "だ")`
    /// returns 6 candidates and Postgres's row order without an
    /// ORDER BY is non-deterministic — `da_first_reading_n` would
    /// otherwise alternate between the `n` reading (seq 1564200) and
    /// the copula (seq 2089020). This case exercises the `cop-da-p`
    /// branch (`(intersection seq-set *copulae*)` truthy) and the
    /// `common = 0` arm of the common-bonus cascade.
    #[tokio::test]
    async fn da_copula_cop_da_p_branch() {
        let ctx = ctx_from_env().await;
        let w = kana_by_seq_text(&ctx, 2089020, "だ").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
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
        let (score, _) = calc_score(&ctx, &w, true, None, None, &[]).await.unwrap();
        assert_eq!(score, 16);
    }

    /// REPL: `(calc-score (car (find-word "食べる")))` →
    ///   `504  (:POSI ("v1" "vt") :SEQ-SET (1358280) :CONJ NIL :COMMON 25 :SCORE-INFO (21 NIL 0 NIL) :KPCL (T T T T))`
    #[tokio::test]
    async fn taberu_root_verb() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "食べる").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
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

    /// REPL: `(calc-score (car (find-word "食べる")) :kanji-break '(0))` →
    ///   `252  (... :SCORE-INFO (21 (0) 0 NIL) ...)`
    /// `kanji_break_penalty` only adjusts the outer score; it does not
    /// mutate any field of `info`. Every non-score-info field is
    /// therefore identical to [`taberu_root_verb`]'s output.
    #[tokio::test]
    async fn taberu_with_kanji_break() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "食べる").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[0]).await.unwrap();
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

    /// REPL: `(calc-score (car (find-word "ありがとう")))` →
    ///   `525  (:POSI ("int") :SEQ-SET (1586820) :CONJ NIL :COMMON 0 :SCORE-INFO (21 NIL 0 NIL) :KPCL (NIL T T T))`
    #[tokio::test]
    async fn arigatou_interjection_long() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "ありがとう").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
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

    /// REPL: `(calc-score (car (find-word "コンピューター")))` →
    ///   `440  (:POSI ("n") :SEQ-SET (1053350) :CONJ NIL :COMMON 0 :SCORE-INFO (11 NIL 0 NIL) :KPCL (T NIL T T))`
    #[tokio::test]
    async fn computer_katakana_path() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "コンピューター").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
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

    /// REPL: `(calc-score (car (find-word "ねこ")) :use-length 5)` →
    ///   `80  (:POSI ("n") :SEQ-SET (1467640) :CONJ NIL :COMMON 7
    ///        :SCORE-INFO (4 NIL 64 NIL) :KPCL (NIL NIL T NIL))`
    /// REPL: `(calc-score (car (find-word "ねこ")) :use-length 3)` →
    ///   `32  (:POSI ("n") :SEQ-SET (1467640) :CONJ NIL :COMMON 7
    ///        :SCORE-INFO (4 NIL 16 NIL) :KPCL (NIL NIL T NIL))`
    /// REPL: `(calc-score (car (find-word "ねこ")) :use-length 2)` →
    ///   `16  (:POSI ("n") :SEQ-SET (1467640) :CONJ NIL :COMMON 7
    ///        :SCORE-INFO (4 NIL 0 NIL) :KPCL (NIL NIL T NIL))`
    #[tokio::test]
    async fn neko_use_length_variations() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "ねこ").await;

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
            .await
            .unwrap();
        assert_eq!(score, 80);
        let info = info.unwrap();
        assert_neko_baseline(&info);
        assert_eq!(info.score_info.use_length_bonus, 64);

        let (score, info) = calc_score(&ctx, &w, false, Some(3), None, &[])
            .await
            .unwrap();
        assert_eq!(score, 32);
        let info = info.unwrap();
        assert_neko_baseline(&info);
        assert_eq!(info.score_info.use_length_bonus, 16);

        let (score, info) = calc_score(&ctx, &w, false, Some(2), None, &[])
            .await
            .unwrap();
        assert_eq!(score, 16);
        let info = info.unwrap();
        assert_neko_baseline(&info);
        assert_eq!(info.score_info.use_length_bonus, 0);
    }

    // ----- Class-C regression: compound-text whose inner calc-score
    //       on `score-base` hits a skip-path and returns `0` with no
    //       info. Upstream `dict.lisp:785-786`:
    //           (multiple-value-bind (score info) (apply 'calc-score args)
    //             (setf (getf info :conj) (word-conj-data reading)) ...)
    //       The `multiple-value-bind` of a single-value return binds
    //       `info` to nil; `(setf (getf nil :conj) X)` is CL's
    //       setf-getf-on-nil idiom, which rewrites the binding to a
    //       fresh plist `(:conj X)`. The Rust port must mirror this
    //       rather than propagating `(0, None)`.
    //
    //       Both rows below come from the chunk_b_segmentation_2026_05_14
    //       parquet — the captured args / result tell us exactly what
    //       outer info plist upstream produced. The current `None =>`
    //       arm at line 167 returns `(0, None)`, so both tests fail
    //       on `info.expect(...)`.

    /// Row 279 of chunk_b calc_score parquet. Compound `れちゃう`,
    /// `score_base = null` (falls back to primary), `score_mod = 5`.
    /// Last word `ちゃう` (seq 2013800, conjugations=:ROOT) — and
    /// `get-conj-data(2013800, :ROOT, "ちゃう")` returns nil on the
    /// real DB, so the outer `(word-conj-data <compound>)` returns nil
    /// and the synthesized plist has `:CONJ null`.
    ///
    /// Captured:
    ///   args   = [<compound>, ":FINAL", null, ":KANJI-BREAK", null]
    ///   result = [0, [":CONJ", null]]
    #[tokio::test]
    async fn compound_skipword_partial_info_conj_null() {
        let ctx = ctx_from_env().await;

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
            .await
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
        // The other five fields default to zero/empty — only `:CONJ`
        // is set by the setf-getf-on-nil synthesis.
        assert!(info.posi.is_empty(), "info.posi: {:?}", info.posi);
        assert!(info.seq_set.is_empty(), "info.seq_set: {:?}", info.seq_set);
        assert_eq!(info.common, None);
        assert_eq!(info.score_info.prop_score, 0);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, false, false, false));
    }

    /// chunk_b calc_score row capturing compound `られなくなりました`,
    /// `score_base = null`, `score_mod = (1 5)` (list). Last word
    /// `なりました` (seq 10374833, conjugations=[378046]) — the outer
    /// `(word-conj-data <compound>)` recurses to it and produces a
    /// single CONJ-DATA: seq=10374833, from=1375610 (なる), conj-prop
    /// (id=386572, conj_id=378046, conj_type=2, fml=true, pos="v5r"),
    /// src_map=[("なりました","なる")].
    ///
    /// Captured:
    ///   result = [0, [":CONJ", [{class:CONJ-DATA, seq:10374833,
    ///            from:1375610, prop:{conj_id:378046, conj_type:2,
    ///            fml:true, neg:null, pos:"v5r"}, src_map:[[なりました,
    ///            なる]]}]]]
    #[tokio::test]
    async fn compound_skipword_partial_info_conj_non_null() {
        let ctx = ctx_from_env().await;

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
            .await
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
        // REPL: SELECT neg, fml FROM conj_prop WHERE id = 386572; → f, t
        // PG false (`f`) decodes to Some(false) at the sqlx layer.
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
