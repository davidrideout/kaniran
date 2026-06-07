mod adjoin_word {
    use crate::dict::accessors::*;
    use crate::dict::dao::KanaText;
    use crate::dict::dao::KanjiText;
    use crate::dict::dao::SimpleText;

    fn kanji(seq: i32, text: &str) -> KanjiText {
        KanjiText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kana: None,
            state: SimpleText::default(),
        }
    }

    fn kana(seq: i32, text: &str) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    // The `:around` text/kana defaults reach `get-kana` which hits
    // the database for kanji-text inputs; the unit tests below stay
    // synchronous by passing explicit `:text` / `:kana` keywords so
    // the `:around` defaulting paths fall through without touching
    // the DB. The text/kana defaulting paths are exercised in the
    // REPL probe `tests/repl_adjoin_concat_defaults` (see the
    // `repl_*` tests below) and pinned against the REPL transcript.

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    // ----- (simple-text, simple-text) primary method -----

    #[tokio::test]
    async fn simple_simple_explicit_text_and_kana() {
        // T2 from REPL probe of upstream ichiran:
        //   (adjoin-word w1 w2 :text "abc" :kana "xyz" :score-mod 7)
        //   => COMPOUND-TEXT text="abc" kana="xyz" score-mod=7
        //                    score-base=NIL words=("食べ" "たい")
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let result = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("abc".into()),
            Some("xyz".into()),
            Some(ScoreMod::Single(7)),
            None,
        )
        .await
        .unwrap();
        assert_eq!(result.text, "abc");
        assert_eq!(result.kana, "xyz");
        assert!(matches!(result.score_mod, ScoreMod::Single(7)));
        assert!(result.score_base.is_none());
        assert_eq!(result.words.len(), 2);
    }

    #[tokio::test]
    async fn simple_simple_primary_is_word1() {
        // T1 from REPL probe of upstream ichiran:
        //   words=("食べ" "たい"); primary is word1.
        // Pinned: result.primary derefs to the word1 input.
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let result = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            None,
            None,
        )
        .await
        .unwrap();
        match &*result.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10092273),
            _ => panic!("primary must be the input word1 (kanji-text)"),
        }
        // Words are [word1, word2].
        assert_eq!(result.words.len(), 2);
    }

    #[tokio::test]
    async fn simple_simple_score_mod_none_defaults_to_zero() {
        // T1 from REPL probe of upstream ichiran:
        //   (adjoin-word w1 w2)  => score-mod=0
        // T8: explicit :score-mod nil => score-mod=0
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let result = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            None,
            None,
        )
        .await
        .unwrap();
        assert!(matches!(result.score_mod, ScoreMod::Single(0)));
    }

    #[tokio::test]
    async fn simple_simple_score_base_passthrough() {
        // T5 from REPL probe of upstream ichiran:
        //   (adjoin-word w1 w2 :score-base w1)
        //   => score-base text="食べ"
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let sb = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let result = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Single(0)),
            Some(sb),
        )
        .await
        .unwrap();
        match result.score_base.as_deref() {
            Some(KaniWordDispatchEnum::Kanji(k)) => assert_eq!(k.text, "食べ"),
            _ => panic!("score-base must carry the kanji-text we passed in"),
        }
    }

    // ----- (compound-text, simple-text) primary method -----

    #[tokio::test]
    async fn compound_simple_appends_words_and_updates_text_kana() {
        // T3 from REPL probe of upstream ichiran:
        //   c3 = (adjoin-word w1 w2 :score-mod 3)  ; "食べたい" / "たべたい"
        //   c3b = (adjoin-word c3 w3 :score-mod 4) ; w3 = "ない"
        //   => c3b text="食べたいない" kana="たべたいない"
        //      words-text=("食べ" "たい" "ない")
        //      score-mod=(4 3)
        //      (eq c3 c3b)=T
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c3 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Single(3)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c3b = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c3),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(4)),
            None,
        )
        .await
        .unwrap();
        assert_eq!(c3b.text, "食べたいない");
        assert_eq!(c3b.kana, "たべたいない");
        assert_eq!(c3b.words.len(), 3);
        // dict.lisp:651 — first compound,simple adjoin: (list new old)
        match &c3b.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 2);
                assert!(matches!(v[0], ScoreMod::Single(4)));
                assert!(matches!(v[1], ScoreMod::Single(3)));
            }
            _ => panic!("score-mod must be Stack([4, 3]) after first compound adjoin"),
        }
    }

    #[tokio::test]
    async fn compound_simple_third_adjoin_grows_stack() {
        // T4 from REPL probe of upstream ichiran:
        //   c4  = (adjoin-word w1 w2 :score-mod 1)   ; single 1
        //   c4b = (adjoin-word c4 w3 :score-mod 2)   ; (2 1)
        //   c4c = (adjoin-word c4b w4 :score-mod 5)  ; (5 2 1)
        //   words-text=("食べ" "たい" "ない" "だ")
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c4 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Single(1)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c4b = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c4),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(2)),
            None,
        )
        .await
        .unwrap();
        let w4 = KaniSimpleTextDispatchEnum::Kana(kana(2089020, "だ"));
        let c4c = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c4b),
            w4,
            Some("食べたいないだ".into()),
            Some("たべたいないだ".into()),
            Some(ScoreMod::Single(5)),
            None,
        )
        .await
        .unwrap();
        assert_eq!(c4c.words.len(), 4);
        match &c4c.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 3);
                assert!(matches!(v[0], ScoreMod::Single(5)));
                assert!(matches!(v[1], ScoreMod::Single(2)));
                assert!(matches!(v[2], ScoreMod::Single(1)));
            }
            _ => panic!("score-mod must be Stack([5, 2, 1]) after third adjoin"),
        }
    }

    #[tokio::test]
    async fn compound_simple_ignores_score_base() {
        // T6 from REPL probe of upstream ichiran:
        //   (adjoin-word w1 w2 :score-mod 1 :score-base w1)  ; sets sb=w1
        //   (adjoin-word c6 w3 :score-mod 2 :score-base w2)  ; sb stays w1
        //   => after 2nd adjoin: score-base text="食べ"
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let sb_w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let c6 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Single(1)),
            Some(sb_w1),
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        // Try to overwrite with :score-base w2 — should be ignored.
        let sb_w2 = KaniWordDispatchEnum::Kana(kana(1406940, "たい"));
        let c6b = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c6),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(2)),
            Some(sb_w2),
        )
        .await
        .unwrap();
        match c6b.score_base.as_deref() {
            Some(KaniWordDispatchEnum::Kanji(k)) => assert_eq!(k.text, "食べ"),
            _ => panic!("score-base must remain the originally-set w1 (Kanji '食べ')"),
        }
    }

    #[tokio::test]
    async fn compound_simple_primary_unchanged() {
        // T9 from REPL probe of upstream ichiran:
        //   After (adjoin-word c9 w3 ...) the primary slot's text is
        //   still "食べ" (the original word1 from the first adjoin),
        //   not w3's "ない".
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c9 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Single(1)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c9b = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c9),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(2)),
            None,
        )
        .await
        .unwrap();
        match &*c9b.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べ");
                assert_eq!(k.seq, 10092273);
            }
            _ => panic!("primary must remain the original word1 kanji-text"),
        }
    }

    // ----- :around default computation (hits DB via get-kana) -----

    #[tokio::test]
    async fn around_defaults_text_and_kana_to_concat() {
        // T1 from REPL probe of upstream ichiran:
        //   w1 = 食べ kanji-text seq 10092273 (get-kana "たべ" via best-kana-conj)
        //   w2 = たい kana-text  seq 1406940
        //   (adjoin-word w1 w2)
        //   => text="食べたい" kana="たべたい" score-mod=0
        //
        // Exercises the `:around` defaulting path: text concatenates
        // get-text outputs, kana concatenates get-kana outputs
        // (kanji-text's best-kana-conj hits the conjugation tables).
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let result = adjoin_word(&ctx, w1, w2, None, None, None, None)
            .await
            .unwrap();
        assert_eq!(result.text, "食べたい");
        assert_eq!(result.kana, "たべたい");
        assert!(matches!(result.score_mod, ScoreMod::Single(0)));
    }

    // ----- ScoreMod::Constant — (constantly N) callsites -----

    #[tokio::test]
    async fn simple_simple_constant_score_mod() {
        // REPL probe of upstream ichiran:
        //   (adjoin-word w1 w2 :text "ab" :kana "ab"
        //                      :score-mod (constantly 360))
        //   => slot = #<FUNCTION>  (functionp ✓, funcall returns 360
        //      regardless of argument).
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let result = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Constant(360)),
            None,
        )
        .await
        .unwrap();
        assert!(matches!(result.score_mod, ScoreMod::Constant(360)));
    }

    #[tokio::test]
    async fn compound_simple_grows_constant_into_list() {
        // REPL probe of upstream ichiran:
        //   c1 = (adjoin-word w1 w2 :score-mod (constantly 360))
        //   c2 = (adjoin-word c1 w3 :score-mod 5)
        //   => c2 score-mod = (5 #<FUNCTION>) — Single(5) prepended
        //      onto the Constant(360).
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c1 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Constant(360)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c2 = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c1),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(5)),
            None,
        )
        .await
        .unwrap();
        match &c2.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 2);
                assert!(matches!(v[0], ScoreMod::Single(5)));
                assert!(matches!(v[1], ScoreMod::Constant(360)));
            }
            _ => panic!("score-mod must be Stack([Single(5), Constant(360)])"),
        }
    }

    #[tokio::test]
    async fn compound_simple_constant_onto_constant() {
        // REPL probe of upstream ichiran:
        //   c1 = (adjoin-word w1 w2 :score-mod (constantly 200))
        //   c2 = (adjoin-word c1 w3 :score-mod (constantly 300))
        //   => c2 score-mod = (#<300> #<200>) — Constant(300) prepended
        //      onto Constant(200).
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c1 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Constant(200)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c2 = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c1),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Constant(300)),
            None,
        )
        .await
        .unwrap();
        match &c2.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 2);
                assert!(matches!(v[0], ScoreMod::Constant(300)));
                assert!(matches!(v[1], ScoreMod::Constant(200)));
            }
            _ => panic!("score-mod must be Stack([Constant(300), Constant(200)])"),
        }
    }

    #[tokio::test]
    async fn compound_simple_third_adjoin_mixes_constants_and_integers() {
        // REPL probe of upstream ichiran:
        //   c1 = (adjoin-word w1 w2 :score-mod (constantly 360))
        //   c2 = (adjoin-word c1 w3 :score-mod 5)
        //   c3 = (adjoin-word c2 w4 :score-mod (constantly 200))
        //   => c3 score-mod = (#<200> 5 #<360>)
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c1 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Constant(360)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c2 = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c1),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(5)),
            None,
        )
        .await
        .unwrap();
        let w4 = KaniSimpleTextDispatchEnum::Kana(kana(2089020, "だ"));
        let c3 = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c2),
            w4,
            Some("食べたいないだ".into()),
            Some("たべたいないだ".into()),
            Some(ScoreMod::Constant(200)),
            None,
        )
        .await
        .unwrap();
        match &c3.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 3);
                assert!(matches!(v[0], ScoreMod::Constant(200)));
                assert!(matches!(v[1], ScoreMod::Single(5)));
                assert!(matches!(v[2], ScoreMod::Constant(360)));
            }
            _ => panic!("score-mod must be Stack([Constant(200), Single(5), Constant(360)])"),
        }
    }
}

mod apply_score_mod {
    use crate::dict::accessors::*;

    // All assertions REPL-pinned against upstream ichiran.
    #[test]
    fn integer_method_positive() {
        // (apply-score-mod 3 10 4) = 120
        assert_eq!(apply_score_mod(&ScoreMod::Single(3), 10, 4), 120);
    }

    #[test]
    fn integer_method_zero_score_mod() {
        // (apply-score-mod 0 10 4) = 0
        assert_eq!(apply_score_mod(&ScoreMod::Single(0), 10, 4), 0);
    }

    #[test]
    fn integer_method_negative_score_mod() {
        // (apply-score-mod -2 5 3) = -30
        assert_eq!(apply_score_mod(&ScoreMod::Single(-2), 5, 3), -30);
    }

    #[test]
    fn integer_method_zero_score() {
        // (apply-score-mod 7 0 5) = 0
        assert_eq!(apply_score_mod(&ScoreMod::Single(7), 0, 5), 0);
    }

    #[test]
    fn integer_method_zero_len() {
        // (apply-score-mod 1 100 0) = 0
        assert_eq!(apply_score_mod(&ScoreMod::Single(1), 100, 0), 0);
    }

    // ----- function method, reached via (constantly N) -----

    #[test]
    fn constant_method_positive() {
        // (apply-score-mod (constantly 360) 10 4) = 360
        assert_eq!(apply_score_mod(&ScoreMod::Constant(360), 10, 4), 360);
    }

    #[test]
    fn constant_method_zero_args() {
        // (apply-score-mod (constantly 200) 0 0) = 200
        assert_eq!(apply_score_mod(&ScoreMod::Constant(200), 0, 0), 200);
    }

    #[test]
    fn constant_method_ignores_score_and_len() {
        // (apply-score-mod (constantly 300) 50 7) = 300
        assert_eq!(apply_score_mod(&ScoreMod::Constant(300), 50, 7), 300);
    }

    #[test]
    fn constant_method_negative() {
        // (apply-score-mod (constantly -5) 10 4) = -5
        assert_eq!(apply_score_mod(&ScoreMod::Constant(-5), 10, 4), -5);
    }

    #[test]
    fn constant_method_zero_value() {
        // (apply-score-mod (constantly 0) 10 4) = 0
        assert_eq!(apply_score_mod(&ScoreMod::Constant(0), 10, 4), 0);
    }

    // ----- list method -----

    #[test]
    fn list_method_two_integer_elements() {
        // (apply-score-mod '(3 4) 10 2) = 140
        // = (3*10*2) + (4*10*2) = 60 + 80 = 140
        assert_eq!(
            apply_score_mod(
                &ScoreMod::Stack(vec![ScoreMod::Single(3), ScoreMod::Single(4)]),
                10,
                2
            ),
            140
        );
    }

    #[test]
    fn list_method_empty() {
        // (apply-score-mod '() 10 2) = 0
        assert_eq!(apply_score_mod(&ScoreMod::Stack(vec![]), 10, 2), 0);
    }

    #[test]
    fn list_method_mixed_signs() {
        // (apply-score-mod '(5 -2 1) 10 3) = 120
        // = (5*10*3) + (-2*10*3) + (1*10*3) = 150 - 60 + 30 = 120
        assert_eq!(
            apply_score_mod(
                &ScoreMod::Stack(vec![
                    ScoreMod::Single(5),
                    ScoreMod::Single(-2),
                    ScoreMod::Single(1),
                ]),
                10,
                3
            ),
            120
        );
    }

    #[test]
    fn list_method_single_integer() {
        // (apply-score-mod '(1) 10 5) = 50
        assert_eq!(
            apply_score_mod(&ScoreMod::Stack(vec![ScoreMod::Single(1)]), 10, 5),
            50
        );
    }

    #[test]
    fn list_method_all_zeros() {
        // (apply-score-mod '(0 0 0) 10 5) = 0
        assert_eq!(
            apply_score_mod(
                &ScoreMod::Stack(vec![
                    ScoreMod::Single(0),
                    ScoreMod::Single(0),
                    ScoreMod::Single(0),
                ]),
                10,
                5
            ),
            0
        );
    }

    // ----- list method, mixed function + integer elements -----

    #[test]
    fn list_constant_then_integer() {
        // (apply-score-mod (list (constantly 360) 5) 10 2) = 460
        // = 360 + (5*10*2) = 360 + 100
        assert_eq!(
            apply_score_mod(
                &ScoreMod::Stack(vec![ScoreMod::Constant(360), ScoreMod::Single(5)]),
                10,
                2
            ),
            460
        );
    }

    #[test]
    fn list_integer_then_constant() {
        // (apply-score-mod (list 3 (constantly 100)) 10 2) = 160
        // = (3*10*2) + 100 = 60 + 100
        assert_eq!(
            apply_score_mod(
                &ScoreMod::Stack(vec![ScoreMod::Single(3), ScoreMod::Constant(100)]),
                10,
                2
            ),
            160
        );
    }

    #[test]
    fn list_two_constants() {
        // (apply-score-mod (list (constantly 200) (constantly 300)) 10 2) = 500
        assert_eq!(
            apply_score_mod(
                &ScoreMod::Stack(vec![ScoreMod::Constant(200), ScoreMod::Constant(300),]),
                10,
                2
            ),
            500
        );
    }

    #[test]
    fn list_constant_then_two_integers() {
        // (apply-score-mod (cons (constantly 360) (list 5 7)) 10 2) = 600
        // = 360 + (5*10*2) + (7*10*2) = 360 + 100 + 140
        assert_eq!(
            apply_score_mod(
                &ScoreMod::Stack(vec![
                    ScoreMod::Constant(360),
                    ScoreMod::Single(5),
                    ScoreMod::Single(7),
                ]),
                10,
                2
            ),
            600
        );
    }

    // ----- Constant method exercised by real ichiran segmentation -----
    //
    // The four upstream `(constantly N)` callsites
    // (`suffix-kudasai`/`sou`/`desu`/`desho` at
    // `dict-grammar.lisp:404, 448, 516, 532`) only fire on specific
    // sentence shapes. The (score, len) arguments below were captured
    // by hooking `apply-score-mod` via `sb-int:encapsulate` and
    // running `(simple-segment <sentence>)` through the production
    // segmenter; every recorded function-typed call is preserved
    // verbatim so a regression on the Constant method's arithmetic is
    // detectable end-to-end, not just on synthetic inputs.

    #[test]
    fn kudasai_real_sentence_taberu_te_kudasai() {
        // "食べてください" → suffix-kudasai (dict-grammar.lisp:404)
        //   (apply-score-mod (constantly 360) 14 4) = 360
        assert_eq!(apply_score_mod(&ScoreMod::Constant(360), 14, 4), 360);
    }

    #[test]
    fn sou_real_sentence_omoshiroi_sou() {
        // "面白そう" → suffix-sou default branch (root ∉ {から, い, 出来},
        // dict-grammar.lisp:448-452 falls through to N=70)
        //   (apply-score-mod (constantly 70) 14 2) = 70
        assert_eq!(apply_score_mod(&ScoreMod::Constant(70), 14, 2), 70);
    }

    #[test]
    fn sou_real_sentence_dekiru_sou() {
        // "出来そう" → suffix-sou produces two segmenter parses:
        //   parse 1 (root not "出来"): (apply-score-mod (constantly 70) 14 2) = 70
        //   parse 2 (root = "出来"):   (apply-score-mod (constantly 100) 9  2) = 100
        assert_eq!(apply_score_mod(&ScoreMod::Constant(70), 14, 2), 70);
        assert_eq!(apply_score_mod(&ScoreMod::Constant(100), 9, 2), 100);
    }

    #[test]
    fn desu_real_sentence_tabenai_desu() {
        // "食べないです" → suffix-desu (dict-grammar.lisp:516)
        //   (apply-score-mod (constantly 200) 21 2) = 200
        assert_eq!(apply_score_mod(&ScoreMod::Constant(200), 21, 2), 200);
    }

    #[test]
    fn desho_real_sentence_tabenai_deshou() {
        // "食べないでしょう" → suffix-desho (dict-grammar.lisp:532).
        // Segmenter explores 6 parses, all hitting the same compound
        // but with different (score, len) pairs. All six return 300
        // because the FUNCTION method ignores score and len.
        assert_eq!(apply_score_mod(&ScoreMod::Constant(300), 4, 3), 300);
        assert_eq!(apply_score_mod(&ScoreMod::Constant(300), 10, 3), 300);
        assert_eq!(apply_score_mod(&ScoreMod::Constant(300), 4, 2), 300);
        assert_eq!(apply_score_mod(&ScoreMod::Constant(300), 10, 2), 300);
        assert_eq!(apply_score_mod(&ScoreMod::Constant(300), 21, 3), 300);
        assert_eq!(apply_score_mod(&ScoreMod::Constant(300), 21, 2), 300);
    }
}

mod get_array {
    use crate::dict::accessors::*;
    use crate::dict::path::SegmentList;
    use crate::dict::path::{PathElement, TopArrayItem};

    fn dummy_payload(score: i32) -> TopArrayItem {
        TopArrayItem {
            score,
            payload: vec![PathElement::SegmentList(SegmentList {
                segments: vec![],
                start: 0,
                end: 0,
                top: None,
                matches: 0,
            })],
        }
    }

    #[test]
    fn empty_top_array_returns_empty_slice() {
        // REPL: empty len=0
        let ta = TopArray::new(3);
        assert_eq!(get_array(&ta).len(), 0);
    }

    #[test]
    fn partial_returns_first_count_slots() {
        // REPL: after 1 register, len=1, first score=50
        let mut ta = TopArray::new(3);
        ta.array[0] = Some(dummy_payload(50));
        ta.count = 1;
        let arr = get_array(&ta);
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].as_ref().unwrap().score, 50);
    }

    #[test]
    fn count_equal_to_len_returns_full() {
        // REPL: after 3, len=3 scores=(100 50 10)
        let mut ta = TopArray::new(3);
        ta.array[0] = Some(dummy_payload(100));
        ta.array[1] = Some(dummy_payload(50));
        ta.array[2] = Some(dummy_payload(10));
        ta.count = 3;
        let arr = get_array(&ta);
        assert_eq!(arr.len(), 3);
        assert_eq!(
            arr.iter()
                .map(|x| x.as_ref().unwrap().score)
                .collect::<Vec<_>>(),
            vec![100, 50, 10]
        );
    }

    #[test]
    fn count_exceeding_len_returns_full() {
        // REPL: after 4 (overflow), len=3 scores=(999 100 50)
        // count exceeds array.len() but we still return the whole array.
        let mut ta = TopArray::new(3);
        ta.array[0] = Some(dummy_payload(999));
        ta.array[1] = Some(dummy_payload(100));
        ta.array[2] = Some(dummy_payload(50));
        ta.count = 4;
        let arr = get_array(&ta);
        assert_eq!(arr.len(), 3);
        assert_eq!(
            arr.iter()
                .map(|x| x.as_ref().unwrap().score)
                .collect::<Vec<_>>(),
            vec![999, 100, 50]
        );
    }
}

mod get_segment_score {
    use crate::dict::accessors::*;
    use crate::dict::dao::KanaText;
    use crate::dict::dao::SimpleText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    // All assertions back-checked via REPL on the .103 SBCL — see
    // `/tmp/probe_gss.lisp` 2026-05-17 run.

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
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

    fn seg(score: Option<i32>) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score,
            info: None,
            top: None,
            text: None,
        }
    }

    #[test]
    fn synergy_returns_score() {
        // REPL: get-segment-score on synergy with score=7 -> 7
        let s = Synergy {
            description: Some("x".into()),
            connector: Some(String::new()),
            score: 7,
            start: 0,
            end: 1,
        };
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::Synergy(&s)),
            Some(7)
        );
    }

    #[test]
    fn segment_returns_score_when_present() {
        // REPL: segment with score=13 -> 13
        let s = seg(Some(13));
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::Segment(&s)),
            Some(13)
        );
    }

    #[test]
    fn segment_returns_none_when_score_unset() {
        // REPL: segment with no score -> NIL
        let s = seg(None);
        assert_eq!(get_segment_score(&KaniSegmentScoreArg::Segment(&s)), None);
    }

    #[test]
    fn empty_segment_list_returns_zero() {
        // REPL: segment-list with no segments -> 0
        let sl = SegmentList {
            segments: Vec::new(),
            start: 0,
            end: 1,
            top: None,
            matches: 0,
        };
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::SegmentList(&sl)),
            Some(0)
        );
    }

    #[test]
    fn segment_list_returns_first_segment_score() {
        // REPL: segment-list with two segs (99, 50) -> 99
        let sl = SegmentList {
            segments: vec![seg(Some(99)), seg(Some(50))],
            start: 0,
            end: 1,
            top: None,
            matches: 0,
        };
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::SegmentList(&sl)),
            Some(99)
        );
    }

    #[test]
    fn segment_list_returns_none_when_first_segment_score_unset() {
        // REPL: segment-list with one nil-score seg -> NIL
        let sl = SegmentList {
            segments: vec![seg(None)],
            start: 0,
            end: 1,
            top: None,
            matches: 0,
        };
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::SegmentList(&sl)),
            None
        );
    }

    #[test]
    fn single_segment_list_returns_that_score() {
        // REPL: segment-list with one seg (42) -> 42
        let sl = SegmentList {
            segments: vec![seg(Some(42))],
            start: 0,
            end: 1,
            top: None,
            matches: 0,
        };
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::SegmentList(&sl)),
            Some(42)
        );
    }
}

mod reading_str {
    use crate::dict::accessors::*;
    use crate::dict::readings::{find_word, FindWordRows};

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn first_simple_text(ctx: &KaniranContext, word: &str) -> KaniSimpleTextDispatchEnum {
        match find_word(ctx, word, false).await.unwrap() {
            FindWordRows::Kanji(rows) => KaniSimpleTextDispatchEnum::Kanji(
                rows.into_iter()
                    .next()
                    .expect("at least one kanji-text row"),
            ),
            FindWordRows::Kana(rows) => KaniSimpleTextDispatchEnum::Kana(
                rows.into_iter().next().expect("at least one kana-text row"),
            ),
        }
    }

    /// REPL fixtures (.103, `(reading-str (car (find-word W)))`), 2026-05-24.
    /// Covers the simple-text method: kanji-text (kanji+kana), kana-text with
    /// a kanji form (get-kanji via best-kanji-conj → 猫), and kana-text without
    /// one (get-kanji nil → bare kana).
    #[tokio::test]
    async fn reading_str_simple_text_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, &str)] = &[
            ("日本", "日本 【にほん】"),
            ("ねこ", "猫 【ねこ】"),
            ("ぴちぴち", "ぴちぴち"),
            ("食べる", "食べる 【たべる】"),
            ("東京", "東京 【とうきょう】"),
        ];
        for (word, expected) in cases {
            let obj = first_simple_text(&ctx, word).await;
            assert_eq!(
                obj.reading_str(&ctx).await.unwrap().as_deref(),
                Some(*expected),
                "word={word}"
            );
        }
    }

    /// REPL fixtures (.103, `(reading-str (car (simple-segment S)))`), 2026-05-24.
    /// Covers the word-info method on real segmented (kanji-type) word-infos.
    #[tokio::test]
    async fn reading_str_word_info_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, &str)] = &[
            ("東京", "東京 【とうきょう】"),
            ("日本語", "日本語 【にほんご】"),
        ];
        for (sentence, expected) in cases {
            let word_infos = crate::dict::word_info::simple_segment(&ctx, sentence, None)
                .await
                .unwrap();
            assert_eq!(
                word_infos[0].reading_str().as_deref(),
                Some(*expected),
                "sentence={sentence}"
            );
        }
    }
}

mod register_item {
    use crate::dict::accessors::*;
    use crate::dict::path::SegmentList;

    fn payload(tag: i32) -> Vec<PathElement> {
        // Use a sentinel SegmentList with `matches` carrying the tag —
        // tests only need a discriminable payload, not real data.
        vec![PathElement::SegmentList(SegmentList {
            segments: vec![],
            start: 0,
            end: 0,
            top: None,
            matches: tag as usize,
        })]
    }

    fn tag_of(item: &TopArrayItem) -> usize {
        match &item.payload[0] {
            PathElement::SegmentList(sl) => sl.matches,
            _ => panic!("unexpected payload variant"),
        }
    }

    fn scores_and_tags(obj: &TopArray) -> Vec<(i32, usize)> {
        obj.array
            .iter()
            .filter_map(|slot| slot.as_ref().map(|item| (item.score, tag_of(item))))
            .collect()
    }

    // REPL probes (`/tmp/probe_register_item.lisp` on .103, 2026-05-19).
    // Tags below correspond to the Lisp keyword payloads :A=1, :B=2,
    // :C=3, :Z=99 — only the structural relationships matter.

    #[test]
    fn a_empty_register_lands_at_index_0() {
        // REPL A: empty limit=3, register score=10 payload=(A).
        // count=1, [0]=(10,A), [1..]=NIL.
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 10, payload(1));
        assert_eq!(ta.count, 1);
        assert_eq!(scores_and_tags(&ta), vec![(10, 1)]);
        assert!(ta.array[1].is_none());
        assert!(ta.array[2].is_none());
    }

    #[test]
    fn b_higher_score_shifts_existing_down() {
        // REPL B: register 10/A then 20/B with limit=3.
        // count=2, [0]=(20,B), [1]=(10,A).
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 10, payload(1));
        register_item(&mut ta, 20, payload(2));
        assert_eq!(ta.count, 2);
        assert_eq!(scores_and_tags(&ta), vec![(20, 2), (10, 1)]);
    }

    #[test]
    fn c_lower_score_appends_after() {
        // REPL C: register 20/A then 10/B with limit=3.
        // count=2, [0]=(20,A), [1]=(10,B).
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 20, payload(1));
        register_item(&mut ta, 10, payload(2));
        assert_eq!(ta.count, 2);
        assert_eq!(scores_and_tags(&ta), vec![(20, 1), (10, 2)]);
    }

    #[test]
    fn d_equal_score_new_lands_below_existing() {
        // REPL D: register 20/A then 20/B — the `>=` test stops descent
        // at the equal slot, new item lands one index lower.
        // count=2, [0]=(20,A), [1]=(20,B).
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 20, payload(1));
        register_item(&mut ta, 20, payload(2));
        assert_eq!(ta.count, 2);
        assert_eq!(scores_and_tags(&ta), vec![(20, 1), (20, 2)]);
    }

    #[test]
    fn e_middle_insert_shifts_lower_down() {
        // REPL E: limit=5, register 30/C, 10/A, 20/B.
        // count=3, [0]=(30,C), [1]=(20,B), [2]=(10,A), [3]=NIL, [4]=NIL.
        let mut ta = TopArray::new(5);
        register_item(&mut ta, 30, payload(3));
        register_item(&mut ta, 10, payload(1));
        register_item(&mut ta, 20, payload(2));
        assert_eq!(ta.count, 3);
        assert_eq!(scores_and_tags(&ta), vec![(30, 3), (20, 2), (10, 1)]);
        assert!(ta.array[3].is_none());
        assert!(ta.array[4].is_none());
    }

    #[test]
    fn f_overflow_at_bottom_is_dropped_silently() {
        // REPL F: limit=3, register 30/C, 20/B, 10/A, 5/Z.
        // 5/Z's idx starts at min(3,3)=3 which fails the `(< idx len)`
        // guard; done is true (prev>=5) → break with no write.
        // count=4, [0..2]=(30,C),(20,B),(10,A); Z dropped.
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 30, payload(3));
        register_item(&mut ta, 20, payload(2));
        register_item(&mut ta, 10, payload(1));
        register_item(&mut ta, 5, payload(99));
        assert_eq!(ta.count, 4);
        assert_eq!(scores_and_tags(&ta), vec![(30, 3), (20, 2), (10, 1)]);
    }

    #[test]
    fn g_full_array_highest_replaces_top_evicts_lowest() {
        // REPL G: limit=3, register 30/C, 20/B, 10/A, then 99/Z.
        // 99/Z walks down through all three; the lowest (10/A) falls off
        // the bottom because idx=3 skip-writes (no out-of-bounds).
        // count=4, [0]=(99,Z), [1]=(30,C), [2]=(20,B).
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 30, payload(3));
        register_item(&mut ta, 20, payload(2));
        register_item(&mut ta, 10, payload(1));
        register_item(&mut ta, 99, payload(99));
        assert_eq!(ta.count, 4);
        assert_eq!(scores_and_tags(&ta), vec![(99, 99), (30, 3), (20, 2)]);
    }

    #[test]
    fn h_limit_one() {
        // REPL H: limit=1. After register 5/A: count=1, [0]=(5,A).
        // After register 10/B: B shifts A out → count=2, [0]=(10,B).
        // After register 3/C: 3 < 10, idx=1 skip-writes, done → count=3,
        // [0] still (10,B).
        let mut ta = TopArray::new(1);
        register_item(&mut ta, 5, payload(1));
        assert_eq!(ta.count, 1);
        assert_eq!(scores_and_tags(&ta), vec![(5, 1)]);
        register_item(&mut ta, 10, payload(2));
        assert_eq!(ta.count, 2);
        assert_eq!(scores_and_tags(&ta), vec![(10, 2)]);
        register_item(&mut ta, 3, payload(3));
        assert_eq!(ta.count, 3);
        assert_eq!(scores_and_tags(&ta), vec![(10, 2)]);
    }

    #[test]
    fn i_limit_zero_increments_count_only() {
        // REPL I: limit=0. No slot can be written; count still increments.
        // count=1, array empty.
        let mut ta = TopArray::new(0);
        register_item(&mut ta, 5, payload(1));
        assert_eq!(ta.count, 1);
        assert_eq!(ta.array.len(), 0);
    }

    #[test]
    fn j_empty_payload() {
        // REPL J: find-best-path's initial call `(register-item top
        // (gap-penalty 0 str-length) nil)` — empty payload is the
        // structural zero element.
        let mut ta = TopArray::new(3);
        register_item(&mut ta, -1000, vec![]);
        assert_eq!(ta.count, 1);
        let item = ta.array[0].as_ref().unwrap();
        assert_eq!(item.score, -1000);
        assert!(item.payload.is_empty());
    }
}

mod word_type {
    use crate::dict::accessors::*;
    use crate::dict::counters::classes::{Common, Counter, CounterSource, CounterText};
    use crate::dict::dao::KanaText;
    use crate::dict::dao::KanjiText;
    use crate::dict::dao::SimpleText;
    use crate::dict::text_classes::ProxyText;
    use crate::dict::text_classes::{CompoundText, ScoreMod};

    fn kanji(seq: i32, text: &str) -> KanjiText {
        KanjiText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kana: None,
            state: SimpleText::default(),
        }
    }

    fn kana(seq: i32, text: &str) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn counter(number_text: &str, text: &str, source: Option<CounterSource>) -> Counter {
        Counter::Base(CounterText {
            text: text.into(),
            kana: String::new(),
            number_text: number_text.into(),
            number: 0,
            source,
            ordinalp: false,
            suffix: None,
            accepts_suffixes: Vec::new(),
            suffix_descriptions: Vec::new(),
            digit_opts: Vec::new(),
            common: Common::Inherit,
            allowed: Vec::new(),
            foreign: false,
        })
    }

    #[test]
    fn kanji_variant_is_kanji() {
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Kanji(kanji(1, "犬"))),
            WordType::Kanji,
        );
    }

    #[test]
    fn kana_variant_is_kana() {
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Kana(kana(1, "いぬ"))),
            WordType::Kana,
        );
    }

    #[test]
    fn counter_with_kanji_in_concatenated_text() {
        // `text` gf concatenates "1" + "人" → "1人"; contains kanji.
        let c = counter("1", "人", None);
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Counter(c)),
            WordType::Kanji,
        );
    }

    #[test]
    fn counter_with_only_kana_text() {
        // "1" + "つ" → "1つ"; no kanji.
        let c = counter("1", "つ", None);
        assert_eq!(word_type(&KaniWordDispatchEnum::Counter(c)), WordType::Kana,);
    }

    #[test]
    fn proxy_recurses_through_source_chain() {
        let leaf = KaniSimpleTextDispatchEnum::Kana(kana(1, "ねこ"));
        let inner = KaniSimpleTextDispatchEnum::Proxy(ProxyText {
            text: String::new(),
            kana: String::new(),
            source: Box::new(leaf),
            state: SimpleText::default(),
        });
        let outer = ProxyText {
            text: String::new(),
            kana: String::new(),
            source: Box::new(inner),
            state: SimpleText::default(),
        };
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Proxy(outer)),
            WordType::Kana,
        );
    }

    #[test]
    fn compound_dispatches_on_primary() {
        let primary = Box::new(KaniWordDispatchEnum::Kanji(kanji(1, "犬")));
        let words = vec![
            KaniWordDispatchEnum::Kanji(kanji(1, "犬")),
            KaniWordDispatchEnum::Kana(kana(2, "ねこ")),
        ];
        let c = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary,
            words,
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Compound(c)),
            WordType::Kanji,
        );
    }
}

mod set_word_conjugations {
    use crate::dict::accessors::*;
    use crate::dict::dao::KanaText;
    use crate::dict::dao::KanjiText;
    use crate::dict::dao::SimpleText;
    use crate::dict::text_classes::ProxyText;
    use crate::dict::text_classes::{CompoundText, ScoreMod};

    fn kana(seq: i32) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn kanji(seq: i32) -> KanjiText {
        KanjiText {
            id: 0,
            seq,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kana: None,
            state: SimpleText::default(),
        }
    }

    #[test]
    fn kana_text_writes_state_slot() {
        let mut w = KaniWordDispatchEnum::Kana(kana(1));
        set_word_conjugations(&mut w, Some(WordConjugations::Root));
        match &w {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.state.conjugations, Some(WordConjugations::Root));
            }
            _ => panic!("expected kana variant"),
        }
    }

    #[test]
    fn kanji_text_writes_state_slot() {
        let mut w = KaniWordDispatchEnum::Kanji(kanji(2));
        set_word_conjugations(&mut w, Some(WordConjugations::Ids(vec![10, 20])));
        match &w {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(
                    k.state.conjugations,
                    Some(WordConjugations::Ids(vec![10, 20]))
                );
            }
            _ => panic!("expected kanji variant"),
        }
    }

    #[test]
    fn kana_text_writes_none() {
        let mut k = kana(3);
        k.state.conjugations = Some(WordConjugations::Root);
        let mut w = KaniWordDispatchEnum::Kana(k);
        set_word_conjugations(&mut w, None);
        match &w {
            KaniWordDispatchEnum::Kana(k) => assert!(k.state.conjugations.is_none()),
            _ => panic!("expected kana variant"),
        }
    }

    #[test]
    fn kanji_text_writes_none() {
        let mut k = kanji(4);
        k.state.conjugations = Some(WordConjugations::Ids(vec![1, 2]));
        let mut w = KaniWordDispatchEnum::Kanji(k);
        set_word_conjugations(&mut w, None);
        match &w {
            KaniWordDispatchEnum::Kanji(k) => assert!(k.state.conjugations.is_none()),
            _ => panic!("expected kanji variant"),
        }
    }

    #[test]
    fn proxy_writes_through_source_chain() {
        // Two-level proxy chain — verify the setter descends to the leaf.
        let leaf = KaniSimpleTextDispatchEnum::Kana(kana(99));
        let inner = ProxyText {
            text: String::new(),
            kana: String::new(),
            source: Box::new(leaf),
            state: SimpleText::default(),
        };
        let outer = ProxyText {
            text: String::new(),
            kana: String::new(),
            source: Box::new(KaniSimpleTextDispatchEnum::Proxy(inner)),
            state: SimpleText::default(),
        };
        let mut w = KaniWordDispatchEnum::Proxy(outer);
        set_word_conjugations(&mut w, Some(WordConjugations::Root));

        // Drill down and verify the leaf kana was mutated, not the proxy wrapper.
        match &w {
            KaniWordDispatchEnum::Proxy(outer) => {
                assert!(outer.state.conjugations.is_none(), "outer state untouched");
                match &*outer.source {
                    KaniSimpleTextDispatchEnum::Proxy(inner) => {
                        assert!(inner.state.conjugations.is_none(), "inner state untouched");
                        match &*inner.source {
                            KaniSimpleTextDispatchEnum::Kana(k) => {
                                assert_eq!(k.state.conjugations, Some(WordConjugations::Root))
                            }
                            _ => panic!("expected kana leaf"),
                        }
                    }
                    _ => panic!("expected proxy inner"),
                }
            }
            _ => panic!("expected proxy outer"),
        }
    }

    #[test]
    fn compound_writes_to_last_word() {
        // dict.lisp:666-667 — setter descends into (car (last (words word))).
        let first = KaniWordDispatchEnum::Kana(kana(1));
        let last = KaniWordDispatchEnum::Kana(kana(2));
        let primary = first.clone();
        let c = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary: Box::new(primary),
            words: vec![first, last],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let mut w = KaniWordDispatchEnum::Compound(c);
        set_word_conjugations(&mut w, Some(WordConjugations::Root));

        match &w {
            KaniWordDispatchEnum::Compound(c) => {
                // First word untouched.
                match &c.words[0] {
                    KaniWordDispatchEnum::Kana(k) => assert!(k.state.conjugations.is_none()),
                    _ => panic!("expected kana first"),
                }
                // Last word mutated.
                match &c.words[1] {
                    KaniWordDispatchEnum::Kana(k) => {
                        assert_eq!(k.state.conjugations, Some(WordConjugations::Root))
                    }
                    _ => panic!("expected kana last"),
                }
            }
            _ => panic!("expected compound variant"),
        }
    }

    #[test]
    fn compound_writes_ids_value() {
        // Ids variant through compound recursion — distinct from Root.
        let first = KaniWordDispatchEnum::Kana(kana(1));
        let last = KaniWordDispatchEnum::Kanji(kanji(2));
        let primary = first.clone();
        let c = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary: Box::new(primary),
            words: vec![first, last],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let mut w = KaniWordDispatchEnum::Compound(c);
        set_word_conjugations(&mut w, Some(WordConjugations::Ids(vec![7, 11])));

        match &w {
            KaniWordDispatchEnum::Compound(c) => match &c.words[1] {
                KaniWordDispatchEnum::Kanji(k) => assert_eq!(
                    k.state.conjugations,
                    Some(WordConjugations::Ids(vec![7, 11]))
                ),
                _ => panic!("expected kanji last"),
            },
            _ => panic!("expected compound variant"),
        }
    }

    #[test]
    fn nested_compound_recurses_through_inner_last() {
        // Outer compound whose last element is itself a compound. The setter
        // should descend recursively (last → last) rather than stopping at
        // the inner compound — catches a regression that treated compound-as-
        // last as a terminal write target.
        let a = KaniWordDispatchEnum::Kana(kana(1));
        let b = KaniWordDispatchEnum::Kana(kana(2));
        let c = KaniWordDispatchEnum::Kana(kana(3));
        let inner = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary: Box::new(b.clone()),
            words: vec![b, c],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let inner_word = KaniWordDispatchEnum::Compound(inner);
        let outer = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary: Box::new(a.clone()),
            words: vec![a, inner_word],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let mut w = KaniWordDispatchEnum::Compound(outer);
        set_word_conjugations(&mut w, Some(WordConjugations::Root));

        match &w {
            KaniWordDispatchEnum::Compound(outer) => {
                // Outer[0] untouched.
                match &outer.words[0] {
                    KaniWordDispatchEnum::Kana(k) => assert!(k.state.conjugations.is_none()),
                    _ => panic!("expected kana outer[0]"),
                }
                // Outer[1] is the inner compound; its [0] untouched, [1] mutated.
                match &outer.words[1] {
                    KaniWordDispatchEnum::Compound(inner) => {
                        match &inner.words[0] {
                            KaniWordDispatchEnum::Kana(k) => {
                                assert!(k.state.conjugations.is_none(), "inner[0] untouched");
                            }
                            _ => panic!("expected kana inner[0]"),
                        }
                        match &inner.words[1] {
                            KaniWordDispatchEnum::Kana(k) => assert_eq!(
                                k.state.conjugations,
                                Some(WordConjugations::Root),
                                "inner[1] is the leaf — must be mutated"
                            ),
                            _ => panic!("expected kana inner[1]"),
                        }
                    }
                    _ => panic!("expected inner compound at outer[1]"),
                }
            }
            _ => panic!("expected outer compound"),
        }
    }

    #[test]
    fn compound_with_proxy_last_descends_through_source() {
        // Compound whose last element is a Proxy — verify the recursion
        // continues into the proxy's source chain rather than stopping at
        // the proxy wrapper.
        let leaf = KaniSimpleTextDispatchEnum::Kana(kana(50));
        let proxy = ProxyText {
            text: String::new(),
            kana: String::new(),
            source: Box::new(leaf),
            state: SimpleText::default(),
        };
        let first = KaniWordDispatchEnum::Kana(kana(1));
        let c = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary: Box::new(first.clone()),
            words: vec![first, KaniWordDispatchEnum::Proxy(proxy)],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let mut w = KaniWordDispatchEnum::Compound(c);
        set_word_conjugations(&mut w, Some(WordConjugations::Root));

        match &w {
            KaniWordDispatchEnum::Compound(c) => match &c.words[1] {
                KaniWordDispatchEnum::Proxy(p) => {
                    assert!(p.state.conjugations.is_none(), "proxy wrapper untouched");
                    match &*p.source {
                        KaniSimpleTextDispatchEnum::Kana(k) => assert_eq!(
                            k.state.conjugations,
                            Some(WordConjugations::Root),
                            "proxy source (leaf) must be mutated"
                        ),
                        _ => panic!("expected kana leaf under proxy"),
                    }
                }
                _ => panic!("expected proxy at compound[1]"),
            },
            _ => panic!("expected compound variant"),
        }
    }

    #[test]
    #[should_panic(expected = "(setf word-conjugations) on compound-text with empty words")]
    fn empty_compound_panics() {
        // `(car (last nil))` = nil; `(setf (word-conjugations nil) v)` raises
        // no-applicable-method upstream. Mirror with a panic so a silent
        // no-op can't mask a future caller that constructs a compound mid-build.
        let c = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary: Box::new(KaniWordDispatchEnum::Kana(kana(1))),
            words: Vec::new(),
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let mut w = KaniWordDispatchEnum::Compound(c);
        set_word_conjugations(&mut w, Some(WordConjugations::Root));
    }

    #[test]
    #[should_panic(expected = "(setf word-conjugations) on counter-text")]
    fn counter_text_panics() {
        use crate::dict::counters::classes::{Common, Counter, CounterText};
        let counter = Counter::Base(CounterText {
            text: String::new(),
            kana: String::new(),
            number_text: "1".into(),
            number: 1,
            source: None,
            ordinalp: false,
            suffix: None,
            accepts_suffixes: Vec::new(),
            suffix_descriptions: Vec::new(),
            digit_opts: Vec::new(),
            common: Common::Inherit,
            allowed: Vec::new(),
            foreign: false,
        });
        let mut w = KaniWordDispatchEnum::Counter(counter);
        set_word_conjugations(&mut w, Some(WordConjugations::Root));
    }
}

mod word_conjugations {
    use crate::conn::kani_context::KaniranContext;
    use crate::dict::accessors::*;
    use crate::dict::dao::KanaText;
    use crate::dict::dao::KanjiText;
    use crate::dict::dao::SimpleText;
    use crate::dict::readings::{find_word, FindWordRows};
    use crate::dict::text_classes::{CompoundText, ScoreMod};

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn kana_with_conj(seq: i32, conj: Option<WordConjugations>) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText {
                conjugations: conj,
                hintedp: false,
            },
        }
    }

    fn kanji_with_conj(seq: i32, conj: Option<WordConjugations>) -> KanjiText {
        KanjiText {
            id: 0,
            seq,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kana: None,
            state: SimpleText {
                conjugations: conj,
                hintedp: false,
            },
        }
    }

    #[test]
    fn kana_text_returns_state_slot() {
        let k = kana_with_conj(1, Some(WordConjugations::Root));
        assert_eq!(
            word_conjugations(&KaniWordDispatchEnum::Kana(k)),
            Some(WordConjugations::Root)
        );
    }

    #[test]
    fn kanji_text_returns_state_slot() {
        let k = kanji_with_conj(2, Some(WordConjugations::Ids(vec![10, 20])));
        assert_eq!(
            word_conjugations(&KaniWordDispatchEnum::Kanji(k)),
            Some(WordConjugations::Ids(vec![10, 20]))
        );
    }

    #[test]
    fn counter_text_always_returns_none() {
        use crate::dict::counters::classes::{Common, Counter, CounterText};
        let counter = Counter::Base(CounterText {
            text: String::new(),
            kana: String::new(),
            number_text: "1".into(),
            number: 1,
            source: None,
            ordinalp: false,
            suffix: None,
            accepts_suffixes: Vec::new(),
            suffix_descriptions: Vec::new(),
            digit_opts: Vec::new(),
            common: Common::Inherit,
            allowed: Vec::new(),
            foreign: false,
        });
        assert!(word_conjugations(&KaniWordDispatchEnum::Counter(counter)).is_none());
    }

    #[test]
    fn proxy_recurses_through_source_chain() {
        use crate::dict::text_classes::ProxyText;
        let leaf =
            KaniSimpleTextDispatchEnum::Kana(kana_with_conj(99, Some(WordConjugations::Root)));
        let inner = ProxyText {
            text: String::new(),
            kana: String::new(),
            source: Box::new(leaf),
            state: SimpleText::default(),
        };
        let outer = ProxyText {
            text: String::new(),
            kana: String::new(),
            source: Box::new(KaniSimpleTextDispatchEnum::Proxy(inner)),
            state: SimpleText::default(),
        };
        assert_eq!(
            word_conjugations(&KaniWordDispatchEnum::Proxy(outer)),
            Some(WordConjugations::Root)
        );
    }

    #[test]
    fn compound_recurses_to_last_word() {
        // dict.lisp:663 — (defmethod word-conjugations ((word compound-text))
        //   (word-conjugations (car (last (words word)))))
        let first = KaniWordDispatchEnum::Kana(kana_with_conj(1, Some(WordConjugations::Root)));
        let last =
            KaniWordDispatchEnum::Kana(kana_with_conj(2, Some(WordConjugations::Ids(vec![5]))));
        let c = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary: Box::new(first.clone()),
            words: vec![first, last],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        assert_eq!(
            word_conjugations(&KaniWordDispatchEnum::Compound(c)),
            Some(WordConjugations::Ids(vec![5]))
        );
    }

    #[tokio::test]
    async fn real_kana_text_row_returns_none_by_default() {
        // A freshly-loaded DB row leaves state.conjugations as None
        // (FromRow resets state to Default). Pins the integration
        // surface: find_word("ねこ") rows have no conjugation
        // annotation until the find-word pipeline sets it.
        let ctx = ctx_from_env().await;
        let rows = find_word(&ctx, "ねこ", false).await.unwrap();
        let row = match rows {
            FindWordRows::Kana(v) => v.into_iter().next().expect("kana row for ねこ"),
            FindWordRows::Kanji(_) => panic!("expected kana rows for ねこ"),
        };
        assert!(word_conjugations(&KaniWordDispatchEnum::Kana(row)).is_none());
    }
}
