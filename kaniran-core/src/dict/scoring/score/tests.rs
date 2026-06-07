mod length_multiplier {
    use crate::dict::scoring::score::*;

    // REPL fixtures (.103, ichiran/dict::length-multiplier), 2026-05-25.
    // `(length, power, len-lim) -> result`; both cond branches and the
    // `length == len-lim` boundary (first branch) are covered.
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

    // REPL-pinned (.103 SBCL, 2026-05-13):
    //   *length-coeff-sequences* =
    //     ((:STRONG 1 8 24 40 60)
    //      (:WEAK   1 4 9 16 25 36)
    //      (:TAIL   4 9 16 24)
    //      (:LTAIL  4 12 18 24))
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

    // All assertions REPL-pinned against upstream ichiran.
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
    //
    // Every assertion REPL-pinned against upstream ichiran 2026-05-16.

    #[tokio::test]
    async fn no_info_above_cutoff_halves_with_ceiling() {
        // REPL: (kanji-break-penalty '(0) 100) → 50
        // 100 >= 5 → max(5, ceil(100/2) + 0) = max(5, 50) = 50
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = kanji_break_penalty(&ctx, &[0], 100, None, "", None, None)
            .await
            .unwrap();
        assert_eq!(got, 50);
    }

    #[tokio::test]
    async fn no_info_odd_score_rounds_up() {
        // REPL: (kanji-break-penalty '(1) 100) → 50 (same arithmetic; end branch unused without posi)
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = kanji_break_penalty(&ctx, &[1], 100, None, "", None, None)
            .await
            .unwrap();
        assert_eq!(got, 50);
    }

    #[tokio::test]
    async fn no_info_both_branch() {
        // REPL: (kanji-break-penalty '(0 5) 100) → 50
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = kanji_break_penalty(&ctx, &[0, 5], 100, None, "", None, None)
            .await
            .unwrap();
        assert_eq!(got, 50);
    }

    #[tokio::test]
    async fn below_cutoff_returns_unchanged() {
        // REPL: (kanji-break-penalty '(0) 4) → 4 (4 < *score-cutoff* = 5)
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = kanji_break_penalty(&ctx, &[0], 4, None, "", None, None)
            .await
            .unwrap();
        assert_eq!(got, 4);
    }

    // ----- info-bearing cases (calc_score + kanji_break_penalty integration) -----
    //
    // The pure-arithmetic cases above exercise the `info=None` arm.
    // These exercise the four cond branches at dict.lisp:709-728 that
    // gate on info contents.

    /// REPL: with seq 1467640 (`猫`, common-rank-7 noun) →
    ///   `(calc-score row)` → 19, info :posi ("n") :seq-set (1467640).
    ///   `(kanji-break-penalty '(0) 19 :info info :text "猫")` → 10.
    ///   Hits the fall-through "penalty applies" branch
    ///   (no seq-set ∩ `*no-kanji-break-penalty*`, no `す` prefix, no
    ///   num/suf/pref bonus). Arithmetic: 19 ≥ 5 → max(5, ceil(19/2) + 0)
    ///   = max(5, 10) = 10.
    #[tokio::test]
    async fn info_fall_through_penalty() {
        use crate::dict::kani_word::KaniWordDispatchEnum;
        use crate::dict::scoring::calc_score::calc_score;
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let rows: Vec<crate::dict::dao::KanjiText> = sqlx::query_as(
            "SELECT * FROM kanji_text WHERE seq = 1467640 AND text = '猫' ORDER BY id LIMIT 1",
        )
        .fetch_all(&ctx.pool)
        .await
        .expect("猫 1467640 row");
        let w = KaniWordDispatchEnum::Kanji(rows.into_iter().next().unwrap());
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        assert_eq!(score, 19);
        let info = info.unwrap();
        let got = kanji_break_penalty(&ctx, &[0], score, Some(&info), "猫", None, None)
            .await
            .unwrap();
        assert_eq!(got, 10);
    }

    /// REPL: `飲む` (seq 1169870) is in `*no-kanji-break-penalty*`,
    /// so `kanji-break-penalty` returns `score` unchanged regardless
    /// of arithmetic. Pinned at score=128 (from `(calc-score …)` on
    /// the kanji row).
    #[tokio::test]
    async fn no_penalty_list_short_circuit() {
        use crate::dict::kani_word::KaniWordDispatchEnum;
        use crate::dict::scoring::calc_score::calc_score;
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let rows: Vec<crate::dict::dao::KanjiText> = sqlx::query_as(
            "SELECT * FROM kanji_text WHERE seq = 1169870 AND text = '飲む' ORDER BY id LIMIT 1",
        )
        .fetch_all(&ctx.pool)
        .await
        .expect("飲む 1169870 row");
        let w = KaniWordDispatchEnum::Kanji(rows.into_iter().next().unwrap());
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        let info = info.unwrap();
        // dict.lisp:709 — intersection seq-set *no-kanji-break-penalty*
        // returns truthy → return score unchanged.
        let got = kanji_break_penalty(&ctx, &[0], score, Some(&info), "飲む", None, None)
            .await
            .unwrap();
        assert_eq!(got, score);
    }

    /// REPL: `好き` (seq 1277450) is in `*no-kanji-break-penalty*`,
    /// short-circuits regardless of text. Also exercises the
    /// `(eql end :beg) (alexandria:starts-with #\す text)` arm —
    /// even if seq-set didn't short-circuit, the `す`-prefix branch
    /// would. Pinned via the seq-set route.
    #[tokio::test]
    async fn suki_seq_short_circuit() {
        use crate::dict::kani_word::KaniWordDispatchEnum;
        use crate::dict::scoring::calc_score::calc_score;
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let rows: Vec<crate::dict::dao::KanjiText> = sqlx::query_as(
            "SELECT * FROM kanji_text WHERE seq = 1277450 AND text = '好き' ORDER BY id LIMIT 1",
        )
        .fetch_all(&ctx.pool)
        .await
        .expect("好き 1277450 row");
        let w = KaniWordDispatchEnum::Kanji(rows.into_iter().next().unwrap());
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        let info = info.unwrap();
        let got_kanji_text =
            kanji_break_penalty(&ctx, &[0], score, Some(&info), "好き", None, None)
                .await
                .unwrap();
        let got_kana_text = kanji_break_penalty(&ctx, &[0], score, Some(&info), "すき", None, None)
            .await
            .unwrap();
        // REPL pinned: both → score unchanged (seq-set short-circuits first).
        assert_eq!(got_kanji_text, score);
        assert_eq!(got_kana_text, score);
    }

    #[tokio::test]
    async fn classify_end_results() {
        // pinned via direct cond evaluation on .103: kanji-break list →
        // (cond ((cdr kb) :both) ((eql (car kb) 0) :beg) (t :end))
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

    // All assertions REPL-pinned against upstream ichiran. Each test
    // sorts the returned Vec before comparing because the upstream
    // Lisp `(:select … :distinct …)` does not impose an ORDER BY,
    // and Postgres is free to return distinct rows in any order.
    fn sorted(mut v: Vec<String>) -> Vec<String> {
        v.sort();
        v
    }

    #[tokio::test]
    async fn taberu_single_seq() {
        // (get-non-arch-posi '(1357400)) → ("v5m" "vt")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400]).await.expect("query");
        assert_eq!(sorted(got), vec!["v5m".to_string(), "vt".to_string()]);
    }

    #[tokio::test]
    async fn no_particle_seq() {
        // (get-non-arch-posi '(2089020)) → ("aux-v" "cop" "cop-da")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[2089020]).await.expect("query");
        assert_eq!(
            sorted(got),
            vec!["aux-v".to_string(), "cop".to_string(), "cop-da".to_string(),]
        );
    }

    #[tokio::test]
    async fn dummy_seq_1000220() {
        // (get-non-arch-posi '(1000220)) → ("adj-na")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1000220]).await.expect("query");
        assert_eq!(sorted(got), vec!["adj-na".to_string()]);
    }

    #[tokio::test]
    async fn hon_noun_seq() {
        // (get-non-arch-posi '(1522150)) → ("ctr" "n" "pref")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1522150]).await.expect("query");
        assert_eq!(
            sorted(got),
            vec!["ctr".to_string(), "n".to_string(), "pref".to_string()]
        );
    }

    #[tokio::test]
    async fn counter_seq_1325880() {
        // (get-non-arch-posi '(1325880)) → ("n")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1325880]).await.expect("query");
        assert_eq!(sorted(got), vec!["n".to_string()]);
    }

    #[tokio::test]
    async fn two_seqs_union() {
        // (get-non-arch-posi '(1357400 2089020))
        //   → ("aux-v" "cop" "cop-da" "v5m" "vt")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400, 2089020])
            .await
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

    #[tokio::test]
    async fn zo_particle_seq() {
        // (get-non-arch-posi '(2029110)) → ("int" "prt")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[2029110]).await.expect("query");
        assert_eq!(sorted(got), vec!["int".to_string(), "prt".to_string()]);
    }

    #[tokio::test]
    async fn unknown_seq_returns_empty() {
        // (get-non-arch-posi '(99999999)) → NIL
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[99999999]).await.expect("query");
        assert!(got.is_empty(), "expected NIL, got {got:?}");
    }

    #[tokio::test]
    async fn empty_seq_set_returns_empty() {
        // (get-non-arch-posi nil) → NIL.
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[]).await.expect("query");
        assert!(got.is_empty(), "expected NIL, got {got:?}");
    }

    #[tokio::test]
    async fn many_seqs_union() {
        // (get-non-arch-posi '(1357400 2089020 1522150 1000220))
        //   → ("adj-na" "aux-v" "cop" "cop-da" "ctr" "n" "pref" "v5m" "vt")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400, 2089020, 1522150, 1000220])
            .await
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

    #[tokio::test]
    async fn taberu_with_conj_root() {
        // (get-non-arch-posi (list 1357400 2027820)) → ("exp" "v5m" "vt")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400, 2027820])
            .await
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

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn first_kana_for(ctx: &KaniranContext, s: &str) -> KaniWordDispatchEnum {
        match find_word(ctx, s, false).await.unwrap() {
            FindWordRows::Kana(mut v) => KaniWordDispatchEnum::Kana(v.remove(0)),
            FindWordRows::Kanji(mut v) => KaniWordDispatchEnum::Kanji(v.remove(0)),
        }
    }

    /// Deterministic single-row fetch — `find-word`'s SQL has no
    /// ORDER BY, so the same lookup can rotate first rows between
    /// runs / databases.
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

    async fn kanji_by_seq_text(ctx: &KaniranContext, seq: i32, text: &str) -> KaniWordDispatchEnum {
        let rows: Vec<crate::dict::dao::KanjiText> = sqlx::query_as(
            "SELECT * FROM kanji_text WHERE seq = $1 AND text = $2 ORDER BY id LIMIT 1",
        )
        .bind(seq)
        .bind(text)
        .fetch_all(&ctx.pool)
        .await
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

    // ----- REPL-pinned cases (.103, 2026-05-16). Captured by running
    //       `(gen-score (make-segment :start 0 :end <n> :word w :text "<txt>"))`
    //       followed by `(segment-score s) / (segment-info s)`. -----

    /// REPL: GEN-SCORE 'ねこ': score=16
    /// info=(:POSI ("n") :SEQ-SET (1467640) :CONJ NIL :COMMON 7
    ///       :SCORE-INFO (4 NIL 0 NIL) :KPCL (NIL NIL T NIL))
    #[tokio::test]
    async fn neko_baseline_writes_score_and_info() {
        let ctx = ctx_from_env().await;
        let w = first_kana_for(&ctx, "ねこ").await;
        let mut seg = make_segment(w, 2, "ねこ");
        gen_score(&ctx, &mut seg, false, &[]).await.unwrap();
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

    /// REPL: with row `(select-dao 'kanji-text (:and (:= 'seq 2698030) (:= 'text "猫")))` →
    ///   `(gen-score (make-segment :start 0 :end 1 :word w :text "猫") :kanji-break '(0))` →
    ///   score=3, info=(:POSI NIL :SEQ-SET (2698030) :CONJ NIL :COMMON NIL
    ///                  :SCORE-INFO (3 (0) 0 NIL) :KPCL (T NIL NIL NIL))
    ///
    /// Deterministic-row helper avoids `find-word`'s no-ORDER-BY
    /// nondeterminism.
    #[tokio::test]
    async fn neko_kanji_break_propagates_through_calc_score() {
        let ctx = ctx_from_env().await;
        let w = kanji_by_seq_text(&ctx, 2698030, "猫").await;
        let mut seg = make_segment(w, 1, "猫");
        gen_score(&ctx, &mut seg, false, &[0]).await.unwrap();
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

    /// REPL: with row `(select-dao 'kana-text (:and (:= 'seq 1290020) (:= 'text "ね")))` →
    ///   `(gen-score (make-segment :start 0 :end 1 :word w :text "ね") :final t)` →
    ///   score=4, info=(:POSI ("n") :SEQ-SET (1290020) :CONJ NIL :COMMON 5
    ///                  :SCORE-INFO (4 NIL 0 NIL) :KPCL (NIL NIL T NIL))
    #[tokio::test]
    async fn ne_final_common_n_branch() {
        let ctx = ctx_from_env().await;
        let w = kana_by_seq_text(&ctx, 1290020, "ね").await;
        let mut seg = make_segment(w, 1, "ね");
        gen_score(&ctx, &mut seg, true, &[]).await.unwrap();
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

    /// REPL: `(length (make-slice))` → 0, `(string= (make-slice) "")` → T
    #[test]
    fn empty_seed() {
        let s = make_slice();
        assert_eq!(s.len(), 0);
        assert_eq!(s, "");
    }
}

mod subseq_slice {
    use crate::dict::scoring::score::*;

    /// REPL: `(subseq-slice nil "あいうえお" 1 3)` → `"いう"` (length 2).
    /// Pins character-offset semantics across multi-byte UTF-8.
    #[test]
    fn character_offsets_multi_byte() {
        let r = subseq_slice(None, "あいうえお", 1, Some(3));
        assert_eq!(r, "いう");
    }

    /// REPL: `(subseq-slice nil "abcde" 0 5)` → `"abcde"`.
    #[test]
    fn full_range_ascii() {
        let r = subseq_slice(None, "abcde", 0, Some(5));
        assert_eq!(r, "abcde");
    }

    /// REPL: `(subseq-slice nil "abcde" 0)` → `"abcde"` (default end).
    #[test]
    fn end_defaults_to_length() {
        let r = subseq_slice(None, "abcde", 0, None);
        assert_eq!(r, "abcde");
    }

    /// REPL: `(subseq-slice nil "abc" 1)` → `"bc"` (default end past start).
    #[test]
    fn end_default_with_offset_start() {
        let r = subseq_slice(None, "abc", 1, None);
        assert_eq!(r, "bc");
    }

    /// REPL: `(subseq-slice nil "hello" 2 2)` → `""` (start == end).
    #[test]
    fn empty_range_when_start_equals_end() {
        let r = subseq_slice(None, "hello", 2, Some(2));
        assert_eq!(r, "");
    }

    /// REPL: passing in an existing slice returns a view of `s` regardless.
    /// `(let ((s (make-slice))) (subseq-slice s "hello" 1 4))` → `"ell"`.
    #[test]
    fn slice_argument_is_ignored() {
        let seed = crate::dict::scoring::score::make_slice();
        let r = subseq_slice(Some(seed), "hello", 1, Some(4));
        assert_eq!(r, "ell");
    }

    /// REPL: `(subseq-slice nil "hello" 4 2)` → assertion failure
    /// `(>= END START)` (END=2, START=4).
    #[test]
    #[should_panic(expected = "subseq-slice: end (2) < start (4)")]
    fn end_less_than_start_panics() {
        let _ = subseq_slice(None, "hello", 4, Some(2));
    }

    /// REPL: `(subseq-slice nil "hello" 0 10)` →
    /// `ERROR: The :DISPLACED-TO array is too small.`
    #[test]
    #[should_panic(expected = "subseq-slice: end (10) > (length s) (5)")]
    fn end_past_length_panics() {
        let _ = subseq_slice(None, "hello", 0, Some(10));
    }

    /// REPL: `(subseq-slice nil "hello" 2 7)` →
    /// `ERROR: The :DISPLACED-TO array is too small.` (start in range,
    /// end past length).
    #[test]
    #[should_panic(expected = "subseq-slice: end (7) > (length s) (5)")]
    fn start_in_range_end_past_length_panics() {
        let _ = subseq_slice(None, "hello", 2, Some(7));
    }

    /// REPL: `(subseq-slice nil "hello" 10 12)` →
    /// `ERROR: The :DISPLACED-TO array is too small.` (both out of
    /// range; rejected via the end-bound check).
    #[test]
    #[should_panic(expected = "subseq-slice: end (12) > (length s) (5)")]
    fn start_and_end_past_length_panics() {
        let _ = subseq_slice(None, "hello", 10, Some(12));
    }

    /// REPL: `(subseq-slice nil "hello" 5 5)` → `""` (start == end ==
    /// length is the upper-edge OK case, no error).
    #[test]
    fn start_equal_to_length_at_end_is_ok() {
        let r = subseq_slice(None, "hello", 5, Some(5));
        assert_eq!(r, "");
    }

    /// REPL: `(subseq-slice nil "hello" 0 5)` → `"hello"` (end ==
    /// length is the upper-edge OK case).
    #[test]
    fn end_equal_to_length_is_ok() {
        let r = subseq_slice(None, "hello", 0, Some(5));
        assert_eq!(r, "hello");
    }
}

mod compare_common {
    use crate::dict::scoring::score::*;
    use CompareCommonResult::*;

    // All assertions REPL-pinned against upstream ichiran. Each value
    // matches the exact Lisp return: branch 1 returns c1 itself, so
    // (compare-common 5 NIL) = 5 (C1(5)); branches 2/3 return T or NIL.
    #[test]
    fn nil_c1_always_nil() {
        // (compare-common NIL <anything>) = NIL.
        for c2 in [None, Some(0), Some(1), Some(2), Some(5), Some(10), Some(-3)] {
            assert_eq!(compare_common(None, c2), Nil);
        }
    }

    #[test]
    fn nil_c2_returns_c1_itself() {
        // (compare-common <integer> NIL) returns c1 (branch 1).
        assert_eq!(compare_common(Some(0), None), C1(0));
        assert_eq!(compare_common(Some(1), None), C1(1));
        assert_eq!(compare_common(Some(2), None), C1(2));
        assert_eq!(compare_common(Some(5), None), C1(5));
        assert_eq!(compare_common(Some(10), None), C1(10));
        assert_eq!(compare_common(Some(-3), None), C1(-3));
    }

    #[test]
    fn zero_c1_only_truthy_when_c2_nil() {
        // (compare-common 0 NIL) = 0 (C1(0), truthy); all others NIL.
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
        // (compare-common <pos> 0) = T (branch 2); otherwise NIL.
        assert_eq!(compare_common(Some(1), Some(0)), True);
        assert_eq!(compare_common(Some(2), Some(0)), True);
        assert_eq!(compare_common(Some(5), Some(0)), True);
        assert_eq!(compare_common(Some(10), Some(0)), True);
        assert_eq!(compare_common(Some(0), Some(0)), Nil);
        assert_eq!(compare_common(Some(-3), Some(0)), Nil);
    }

    #[test]
    fn positive_pair_lt_predicate() {
        // Branch 3: (compare-common 1 2) = T (since 1 < 2);
        // (compare-common 2 1) = NIL (since 2 not < 1).
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
        // (compare-common -3 1) = NIL — c1 not > 0, cond falls off.
        assert_eq!(compare_common(Some(-3), Some(1)), Nil);
        assert_eq!(compare_common(Some(-3), Some(2)), Nil);
        assert_eq!(compare_common(Some(-3), Some(5)), Nil);
        assert_eq!(compare_common(Some(-3), Some(10)), Nil);
        assert_eq!(compare_common(Some(-3), Some(-3)), Nil);
        // (compare-common <any> -3) when c2 != 0: third clause requires
        // c1 > 0, so c1<0 falls off; c1>0 returns (< c1 -3) = NIL for
        // any positive c1.
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

    // REPL T1: (cull-segments nil) => NIL.
    #[test]
    fn empty_input_returns_empty() {
        let out = cull_segments(Vec::new());
        assert!(out.is_empty());
    }

    // REPL T2: single segment passes through.
    //   IN: [(score 10)] -> OUT: [(score 10)]
    #[test]
    fn single_segment_passes_through() {
        let out = cull_segments(vec![seg(1, 10, None)]);
        assert_eq!(scores(&out), vec![10]);
        assert_eq!(seqs(&out), vec![1]);
    }

    // REPL T3: descending scores with culling.
    //   IN scores [20, 15, 9, 8] -> max=20 cutoff=10 -> OUT [20, 15].
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

    // REPL T4: identical scores — none culled, order preserved.
    //   IN scores [10, 10, 10] -> OUT [10, 10, 10].
    #[test]
    fn identical_scores_none_culled() {
        let out = cull_segments(vec![seg(1, 10, None), seg(2, 10, None), seg(3, 10, None)]);
        assert_eq!(scores(&out), vec![10, 10, 10]);
        assert_eq!(seqs(&out), vec![1, 2, 3]);
    }

    // REPL T5: unsorted input sorted by score desc.
    //   IN scores [5, 20, 15, 12] -> sorted [20, 15, 12, 5] -> max=20
    //   cutoff=10 -> OUT [20, 15, 12].
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

    // REPL T6: same score, varying :common — compare-common is the
    // primary sort key but score (all equal) is the secondary.
    // Input order [nil, 0, 10, 5] (commons), all score=10.
    //   Expected sorted by compare-common then stable score:
    //   [5, 10, 0, nil] per REPL.
    #[test]
    fn same_score_varying_commons() {
        let out = cull_segments(vec![
            seg(1, 10, Some(None)),
            seg(2, 10, Some(Some(0))),
            seg(3, 10, Some(Some(10))),
            seg(4, 10, Some(Some(5))),
        ]);
        assert_eq!(scores(&out), vec![10, 10, 10, 10]);
        // REPL output order: commons [5, 10, 0, nil] -> seqs [4, 3, 2, 1].
        assert_eq!(seqs(&out), vec![4, 3, 2, 1]);
    }

    // REPL T7: boundary — max=10 cutoff=5; score 5 stays (>= 5), 4
    // dropped.
    //   IN [10, 5, 4] -> OUT [10, 5].
    #[test]
    fn boundary_cutoff_equal_kept() {
        let out = cull_segments(vec![seg(1, 10, None), seg(2, 5, None), seg(3, 4, None)]);
        assert_eq!(scores(&out), vec![10, 5]);
    }

    // REPL T8: odd boundary — max=11 cutoff=11/2=5.5; 6 stays, 5
    // dropped.
    //   IN [11, 6, 5] -> OUT [11, 6].
    #[test]
    fn odd_boundary_cutoff_strict() {
        let out = cull_segments(vec![seg(1, 11, None), seg(2, 6, None), seg(3, 5, None)]);
        assert_eq!(scores(&out), vec![11, 6]);
    }

    // REPL T9: odd boundary with 5 below 5.5.
    //   IN [11, 5] -> OUT [11].
    #[test]
    fn odd_boundary_drops_below_half() {
        let out = cull_segments(vec![seg(1, 11, None), seg(2, 5, None)]);
        assert_eq!(scores(&out), vec![11]);
    }

    // REPL T10: max=10 cutoff=5; score 5 kept.
    //   IN [10, 5] -> OUT [10, 5].
    #[test]
    fn even_boundary_keeps_exactly_half() {
        let out = cull_segments(vec![seg(1, 10, None), seg(2, 5, None)]);
        assert_eq!(scores(&out), vec![10, 5]);
    }

    // REPL T11: zero scores — cutoff 0, all kept (0 >= 0).
    //   IN [0, 0] -> OUT [0, 0].
    #[test]
    fn zero_scores_all_kept() {
        let out = cull_segments(vec![seg(1, 0, None), seg(2, 0, None)]);
        assert_eq!(scores(&out), vec![0, 0]);
    }

    // REPL T12: negative scores — max=-5 cutoff=-2.5; -5 NOT >= -2.5
    // so loop terminates at first segment.
    //   IN [-10, -5] -> sorted [-5, -10] -> OUT [].
    #[test]
    fn negative_scores_all_culled() {
        let out = cull_segments(vec![seg(1, -10, None), seg(2, -5, None)]);
        assert!(out.is_empty());
    }

    // REPL T13: compare-common ordering on commons [nil, 5, 0, 3]
    // with all score=10. Result order (commons): [3, 5, 0, nil] per
    // REPL probe — exercises every compare-common branch:
    //   - 3 < 5 (third clause T)
    //   - 5 < 0 (second clause T)
    //   - 0 < nil (first clause returns 0, truthy)
    //   - nil never sorts before anything (first clause returns nil).
    #[test]
    fn compare_common_ordering_full() {
        let out = cull_segments(vec![
            seg(1, 10, Some(None)),
            seg(2, 10, Some(Some(5))),
            seg(3, 10, Some(Some(0))),
            seg(4, 10, Some(Some(3))),
        ]);
        // REPL order: commons [3, 5, 0, nil] -> seqs [4, 2, 3, 1].
        assert_eq!(seqs(&out), vec![4, 2, 3, 1]);
    }
}
