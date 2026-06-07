mod get_senses_raw {
    use crate::dict::senses::*;
    // All expected values pinned against .103 REPL runs of
    // `(get-senses-raw <seq>)`. Test threads must be 1 —
    // `cargo test --test-threads=1` per the project's DB-test
    // convention.

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[tokio::test]
    async fn unknown_seq_returns_empty() {
        // REPL: (get-senses-raw 999999) => NIL
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 999999).await.unwrap();
        assert_eq!(result, Vec::<RawSense>::new());
    }

    #[tokio::test]
    async fn simple_single_sense() {
        // REPL: (get-senses-raw 1582710)
        // => ((:ORD 0 :GLOSS "Japan" :PROPS (("pos" "n"))))
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1582710).await.unwrap();
        assert_eq!(
            result,
            vec![RawSense {
                ord: 0,
                gloss: "Japan".to_string(),
                props: vec![("pos".to_string(), vec!["n".to_string()])],
            }]
        );
    }

    #[tokio::test]
    async fn multi_value_pos_single_sense() {
        // REPL: (get-senses-raw 1577900)
        // => ((:ORD 0 :GLOSS "eternity" :PROPS (("pos" "adj-no" "n"))))
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1577900).await.unwrap();
        assert_eq!(
            result,
            vec![RawSense {
                ord: 0,
                gloss: "eternity".to_string(),
                props: vec![(
                    "pos".to_string(),
                    vec!["adj-no".to_string(), "n".to_string()],
                )],
            }]
        );
    }

    #[tokio::test]
    async fn field_tag_present() {
        // REPL: (get-senses-raw 1001390)
        // => ((:ORD 0 :GLOSS "oden; dish of various ingredients, e.g.
        //      egg, daikon, potato, chikuwa, konnyaku stewed in
        //      soy-flavored dashi"
        //     :PROPS (("pos" "n") ("field" "food"))))
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1001390).await.unwrap();
        assert_eq!(
            result,
            vec![RawSense {
                ord: 0,
                gloss: "oden; dish of various ingredients, e.g. egg, daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi".to_string(),
                props: vec![
                    ("pos".to_string(), vec!["n".to_string()]),
                    ("field".to_string(), vec!["food".to_string()]),
                ],
            }]
        );
    }

    #[tokio::test]
    async fn stagk_tag_and_multiple_pos() {
        // REPL: (get-senses-raw 1000300)
        // => ((:ORD 0 :GLOSS "to treat; to handle; to deal with"
        //      :PROPS (("stagk" "遇う") ("pos" "v5u" "vt")))
        //     (:ORD 1 :GLOSS "to arrange; to decorate (with); to adorn
        //      (with); to dress (with); to garnish (with)"
        //      :PROPS (("pos" "vt" "v5u"))))
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1000300).await.unwrap();
        assert_eq!(
            result,
            vec![
                RawSense {
                    ord: 0,
                    gloss: "to treat; to handle; to deal with".to_string(),
                    props: vec![
                        ("stagk".to_string(), vec!["遇う".to_string()]),
                        (
                            "pos".to_string(),
                            vec!["v5u".to_string(), "vt".to_string()],
                        ),
                    ],
                },
                RawSense {
                    ord: 1,
                    gloss: "to arrange; to decorate (with); to adorn (with); to dress (with); to garnish (with)".to_string(),
                    props: vec![(
                        "pos".to_string(),
                        vec!["vt".to_string(), "v5u".to_string()],
                    )],
                },
            ]
        );
    }

    #[tokio::test]
    async fn final_group_bag_not_reversed_asymmetry() {
        // REPL: (get-senses-raw 1011960)
        // Pins the upstream asymmetry: sense 1's `stagr` is
        // ("ボタボタ" "ぼたぼた") — the in-loop `(reverse bag)`
        // path; sense 2's `stagr` is ("ぼたぼた" "ボタボタ") — the
        // `finally` path without reverse. Same two sense_prop.ord
        // 0/1 rows in both senses.
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1011960).await.unwrap();
        assert_eq!(
            result,
            vec![
                RawSense {
                    ord: 0,
                    gloss: "dripping; trickling; drop by drop; in drops".to_string(),
                    props: vec![(
                        "pos".to_string(),
                        vec!["adv".to_string(), "adv-to".to_string(), "vs".to_string()],
                    )],
                },
                RawSense {
                    ord: 1,
                    gloss: "wet and heavy (snow, clay, etc.)".to_string(),
                    props: vec![
                        (
                            "stagr".to_string(),
                            vec!["ボタボタ".to_string(), "ぼたぼた".to_string()],
                        ),
                        (
                            "pos".to_string(),
                            vec!["adv".to_string(), "adv-to".to_string(), "vs".to_string()],
                        ),
                    ],
                },
                RawSense {
                    ord: 2,
                    gloss: "(moving) slowly".to_string(),
                    props: vec![
                        (
                            "stagr".to_string(),
                            vec!["ぼたぼた".to_string(), "ボタボタ".to_string()],
                        ),
                        (
                            "pos".to_string(),
                            vec!["adv".to_string(), "adv-to".to_string()],
                        ),
                    ],
                },
            ]
        );
    }

    #[tokio::test]
    async fn sense_with_no_props_yields_empty_props() {
        // REPL: (get-senses-raw 1447690)
        // => ((:ORD 0 :GLOSS "Tokyo" :PROPS (("pos" "n")))
        //     (:ORD 1 :GLOSS "Tokyo Metropolis" :PROPS NIL))
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1447690).await.unwrap();
        assert_eq!(
            result,
            vec![
                RawSense {
                    ord: 0,
                    gloss: "Tokyo".to_string(),
                    props: vec![("pos".to_string(), vec!["n".to_string()])],
                },
                RawSense {
                    ord: 1,
                    gloss: "Tokyo Metropolis".to_string(),
                    props: Vec::new(),
                },
            ]
        );
    }
}

mod get_senses {
    use crate::dict::senses::*;
    // All expected values pinned against .103 REPL runs of
    // `(ichiran/dict::get-senses <seq>)`. Run with `--test-threads=1`.

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[tokio::test]
    async fn unknown_seq_returns_empty() {
        // REPL: (get-senses 999999) => NIL
        let ctx = ctx_from_env().await;
        let result = get_senses(&ctx, 999999).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn simple_single_sense() {
        // REPL: (get-senses 1582710)
        // => (("[n]" "Japan" (("pos" "n"))))
        let ctx = ctx_from_env().await;
        let result = get_senses(&ctx, 1582710).await.unwrap();
        assert_eq!(
            result,
            vec![(
                "[n]".to_string(),
                "Japan".to_string(),
                vec![("pos".to_string(), vec!["n".to_string()])],
            )]
        );
    }

    #[tokio::test]
    async fn multi_value_pos() {
        // REPL: (get-senses 1577900)
        // => (("[adj-no,n]" "eternity" (("pos" "adj-no" "n"))))
        let ctx = ctx_from_env().await;
        let result = get_senses(&ctx, 1577900).await.unwrap();
        assert_eq!(
            result,
            vec![(
                "[adj-no,n]".to_string(),
                "eternity".to_string(),
                vec![(
                    "pos".to_string(),
                    vec!["adj-no".to_string(), "n".to_string()]
                )],
            )]
        );
    }

    #[tokio::test]
    async fn field_tag_preserved_in_props() {
        // REPL: (get-senses 1001390)
        // => (("[n]"
        //       "oden; dish of various ingredients, e.g. egg, daikon,
        //        potato, chikuwa, konnyaku stewed in soy-flavored dashi"
        //       (("pos" "n") ("field" "food"))))
        let ctx = ctx_from_env().await;
        let result = get_senses(&ctx, 1001390).await.unwrap();
        assert_eq!(
            result,
            vec![(
                "[n]".to_string(),
                "oden; dish of various ingredients, e.g. egg, daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi".to_string(),
                vec![
                    ("pos".to_string(), vec!["n".to_string()]),
                    ("field".to_string(), vec!["food".to_string()]),
                ],
            )]
        );
    }

    #[tokio::test]
    async fn second_sense_no_pos_yields_empty_brackets() {
        // REPL: (get-senses 1447690)
        // => (("[n]" "Tokyo" (("pos" "n")))
        //     ("[]" "Tokyo Metropolis" NIL))
        let ctx = ctx_from_env().await;
        let result = get_senses(&ctx, 1447690).await.unwrap();
        assert_eq!(
            result,
            vec![
                (
                    "[n]".to_string(),
                    "Tokyo".to_string(),
                    vec![("pos".to_string(), vec!["n".to_string()])],
                ),
                ("[]".to_string(), "Tokyo Metropolis".to_string(), Vec::new(),),
            ]
        );
    }
}

mod get_senses_str {
    use crate::dict::senses::*;
    // All expected values pinned against .103 REPL runs of
    // `(ichiran/dict::get-senses-str <seq>)`. Run with `--test-threads=1`.

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[tokio::test]
    async fn unknown_seq_yields_empty_string() {
        // REPL: (get-senses-str 999999) => ""
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 999999).await.unwrap();
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn simple_single_sense() {
        // REPL: (get-senses-str 1582710) => "1. [n] Japan"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1582710).await.unwrap();
        assert_eq!(result, "1. [n] Japan");
    }

    #[tokio::test]
    async fn multi_value_pos() {
        // REPL: (get-senses-str 1577900) => "1. [adj-no,n] eternity"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1577900).await.unwrap();
        assert_eq!(result, "1. [adj-no,n] eternity");
    }

    #[tokio::test]
    async fn field_braced_before_gloss() {
        // REPL: (get-senses-str 1001390) =>
        //   "1. [n] {food} oden; dish of various ingredients, e.g. egg,
        //    daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1001390).await.unwrap();
        assert_eq!(
            result,
            "1. [n] {food} oden; dish of various ingredients, e.g. egg, daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi"
        );
    }

    #[tokio::test]
    async fn multi_field_joined_by_comma() {
        // REPL: (get-senses-str 1014100) => "1. [n] {physics,chem} isotope"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1014100).await.unwrap();
        assert_eq!(result, "1. [n] {physics,chem} isotope");
    }

    #[tokio::test]
    async fn s_inf_in_double_angle_brackets() {
        // REPL: (get-senses-str 900000) =>
        //   "1. [suf] 《after the -masu stem of a verb》 to seem to want to (do something)"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 900000).await.unwrap();
        assert_eq!(
            result,
            "1. [suf] 《after the -masu stem of a verb》 to seem to want to (do something)"
        );
    }

    #[tokio::test]
    async fn field_and_s_inf_both_present() {
        // REPL: (get-senses-str 1005660) =>
        //   "1. [n] {food} 《from the sound of the dish being prepared》 shabu-shabu; …"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1005660).await.unwrap();
        assert_eq!(
            result,
            "1. [n] {food} 《from the sound of the dish being prepared》 shabu-shabu; hot pot dish where thinly sliced meat is boiled quickly and then dipped in sauce"
        );
    }

    #[tokio::test]
    async fn multi_sense_separated_by_newline_no_trailing() {
        // REPL: (get-senses-str 1447690) =>
        //   "1. [n] Tokyo\n2. [n] Tokyo Metropolis"
        // sense 2 has pos "[]" → rpos inherits "[n]" from sense 1.
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1447690).await.unwrap();
        assert_eq!(result, "1. [n] Tokyo\n2. [n] Tokyo Metropolis");
    }

    #[tokio::test]
    async fn three_senses_mixed_props() {
        // REPL: (get-senses-str 1011960) =>
        //   "1. [adv,adv-to,vs] dripping; trickling; drop by drop; in drops
        //    2. [adv,adv-to,vs] wet and heavy (snow, clay, etc.)
        //    3. [adv,adv-to] (moving) slowly"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1011960).await.unwrap();
        assert_eq!(
            result,
            "1. [adv,adv-to,vs] dripping; trickling; drop by drop; in drops\n2. [adv,adv-to,vs] wet and heavy (snow, clay, etc.)\n3. [adv,adv-to] (moving) slowly"
        );
    }

    #[tokio::test]
    async fn five_senses_with_s_inf_subset() {
        // REPL: (get-senses-str 1000090) — pinned 5-sense output.
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1000090).await.unwrap();
        assert_eq!(
            result,
            "1. [n] 《sometimes used for zero》 circle\n2. [n] 《when marking a test, homework, etc.》 \"correct\"; \"good\"\n3. [unc] 《placeholder used to censor individual characters or indicate a space to be filled in》 *; _\n4. [n] period; full stop\n5. [n] handakuten (diacritic)"
        );
    }
}

mod match_kana_kanji {
    use crate::dict::dao::KanaText;
    use crate::dict::dao::KanjiText;
    use crate::dict::dao::SimpleText;
    use crate::dict::senses::*;

    fn kana(text: &str, nokanji: bool) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn kanji(text: &str) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kanji(KanjiText {
            id: 0,
            seq: 0,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kana: None,
            state: SimpleText::default(),
        })
    }

    fn restr(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(r, t)| (r.to_string(), t.to_string()))
            .collect()
    }

    /// REPL fixtures (.103, `ichiran/dict::match-kana-kanji`), 2026-05-24.
    /// Readings are seq-1339160 forms: だし (nokanji nil), ダシ
    /// (nokanji t), 出し / 出汁 kanji.
    #[test]
    fn match_kana_kanji_fixtures() {
        let dashi = kana("だし", false);
        let dashi_kata = kana("ダシ", true);
        let dashi_k = kanji("出し");
        let dashi_kanji = kanji("出汁");

        let cases: &[(
            &KaniWordDispatchEnum,
            &KaniWordDispatchEnum,
            Vec<(String, String)>,
            Option<MatchKanaKanjiResult>,
        )] = &[
            // restricted empty → t
            (
                &dashi,
                &dashi_k,
                restr(&[]),
                Some(MatchKanaKanjiResult::Yes),
            ),
            // restr has だし→出し, kanji is 出し → found
            (
                &dashi,
                &dashi_k,
                restr(&[("だし", "出し")]),
                Some(MatchKanaKanjiResult::Found("出し".into())),
            ),
            // restr has だし→出汁, kanji is 出し → not found
            (&dashi, &dashi_k, restr(&[("だし", "出汁")]), None),
            // restr keyed on ダシ only → filters to empty → t
            (
                &dashi,
                &dashi_k,
                restr(&[("ダシ", "出し")]),
                Some(MatchKanaKanjiResult::Yes),
            ),
            // two rows for だし; kanji 出汁 matches the 2nd → found
            (
                &dashi,
                &dashi_kanji,
                restr(&[("だし", "出し"), ("だし", "出汁")]),
                Some(MatchKanaKanjiResult::Found("出汁".into())),
            ),
            // two rows but kanji 出汁 absent among the だし-keyed ones → not found
            (
                &dashi,
                &dashi_kanji,
                restr(&[("だし", "出し"), ("ダシ", "出汁")]),
                None,
            ),
            // nokanji kana reading → nil regardless of restricted
            (&dashi_kata, &dashi_k, restr(&[("ダシ", "出し")]), None),
            (&dashi_kata, &dashi_k, restr(&[]), None),
        ];
        for (kana_reading, kanji_reading, restricted, expected) in cases {
            assert_eq!(
                &match_kana_kanji(kana_reading, kanji_reading, restricted),
                expected,
                "kana={:?} kanji={:?} restricted={:?}",
                text(kana_reading),
                text(kanji_reading),
                restricted,
            );
        }
    }
}

mod match_sense_restrictions {
    use crate::dict::senses::get_senses_raw;
    use crate::dict::senses::*;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn props_of(ctx: &KaniranContext, seq: i32, ord: i32) -> Vec<(String, Vec<String>)> {
        get_senses_raw(ctx, seq)
            .await
            .unwrap()
            .into_iter()
            .find(|s| s.ord == ord)
            .expect("sense ord present")
            .props
    }

    async fn reading_of(
        ctx: &KaniranContext,
        seq: i32,
        text: &str,
        kanji: bool,
    ) -> KaniWordDispatchEnum {
        if kanji {
            let row: KanjiText =
                sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 AND text = $2")
                    .bind(seq)
                    .bind(text)
                    .fetch_one(&ctx.pool)
                    .await
                    .unwrap();
            KaniWordDispatchEnum::Kanji(row)
        } else {
            let row: KanaText =
                sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = $2")
                    .bind(seq)
                    .bind(text)
                    .fetch_one(&ctx.pool)
                    .await
                    .unwrap();
            KaniWordDispatchEnum::Kana(row)
        }
    }

    /// REPL fixtures (.103, `ichiran/dict::match-sense-restrictions`), 2026-05-24.
    /// Covers every cond clause: no-restriction (1339160 s0), direct
    /// member (出し / ダシ / 遇う / ボタボタ / 風太郎), `:kanji`-only nil
    /// (配う), `:kana`-only nil (ポタポタ), the `:kanji` select-dao branch
    /// (出端→t, 出汁→nil via the nokanji ダシ row), and the `:kana`
    /// select-dao branch returning t (だし / あしらう) and the matched
    /// kanji string (ぷうたろう / ふうたろう → "風太郎"; プーたろう → nil;
    /// プータロー nokanji → nil).
    #[tokio::test]
    async fn match_sense_restrictions_fixtures() {
        let ctx = ctx_from_env().await;

        struct Case {
            seq: i32,
            ord: i32,
            text: &'static str,
            kanji: bool,
            expected: Option<MatchKanaKanjiResult>,
        }
        let yes = || Some(MatchKanaKanjiResult::Yes);
        let found = |s: &str| Some(MatchKanaKanjiResult::Found(s.to_string()));
        let cases = [
            // no stags → t (every reading)
            Case {
                seq: 1339160,
                ord: 0,
                text: "出し",
                kanji: true,
                expected: yes(),
            },
            Case {
                seq: 1339160,
                ord: 0,
                text: "だし",
                kanji: false,
                expected: yes(),
            },
            // both stags (stagk 出し, stagr ダシ)
            Case {
                seq: 1339160,
                ord: 1,
                text: "出し",
                kanji: true,
                expected: yes(),
            }, // member stagk
            Case {
                seq: 1339160,
                ord: 1,
                text: "ダシ",
                kanji: false,
                expected: yes(),
            }, // member stagr
            Case {
                seq: 1339160,
                ord: 1,
                text: "出汁",
                kanji: true,
                expected: None,
            }, // :kanji branch, only ダシ(nokanji) matches → nil
            Case {
                seq: 1339160,
                ord: 1,
                text: "だし",
                kanji: false,
                expected: yes(),
            }, // :kana branch → t
            // only stagk (遇う)
            Case {
                seq: 1000300,
                ord: 0,
                text: "遇う",
                kanji: true,
                expected: yes(),
            }, // member stagk
            Case {
                seq: 1000300,
                ord: 0,
                text: "配う",
                kanji: true,
                expected: None,
            }, // :kanji-only ∧ kanji → nil
            Case {
                seq: 1000300,
                ord: 0,
                text: "あしらう",
                kanji: false,
                expected: yes(),
            }, // :kana branch → t
            // only stagr (ボタボタ / ぼたぼた)
            Case {
                seq: 1011960,
                ord: 1,
                text: "ボタボタ",
                kanji: false,
                expected: yes(),
            }, // member stagr
            Case {
                seq: 1011960,
                ord: 1,
                text: "ポタポタ",
                kanji: false,
                expected: None,
            }, // :kana-only ∧ kana → nil
            // both stags, :kanji branch returning t (1580140 stagr でばな non-nokanji)
            Case {
                seq: 1580140,
                ord: 0,
                text: "出端",
                kanji: true,
                expected: yes(),
            },
            // only stagk (風太郎) with restricted_readings rows → Found(string)
            Case {
                seq: 1115120,
                ord: 1,
                text: "風太郎",
                kanji: true,
                expected: yes(),
            }, // member stagk
            Case {
                seq: 1115120,
                ord: 1,
                text: "プー太郎",
                kanji: true,
                expected: None,
            }, // :kanji-only ∧ kanji → nil
            Case {
                seq: 1115120,
                ord: 1,
                text: "プータロー",
                kanji: false,
                expected: None,
            }, // nokanji kana → match-kana-kanji nil
            Case {
                seq: 1115120,
                ord: 1,
                text: "ぷうたろう",
                kanji: false,
                expected: found("風太郎"),
            },
            Case {
                seq: 1115120,
                ord: 1,
                text: "ふうたろう",
                kanji: false,
                expected: found("風太郎"),
            },
            Case {
                seq: 1115120,
                ord: 1,
                text: "プーたろう",
                kanji: false,
                expected: None,
            }, // restr keyed プー太郎, 風太郎 absent → nil
        ];

        for case in &cases {
            let props = props_of(&ctx, case.seq, case.ord).await;
            let reading = reading_of(&ctx, case.seq, case.text, case.kanji).await;
            let actual = match_sense_restrictions(&ctx, case.seq, &props, &reading)
                .await
                .unwrap();
            assert_eq!(
                actual, case.expected,
                "seq={} ord={} text={}",
                case.seq, case.ord, case.text,
            );
        }
    }
}

mod split_pos {
    use crate::dict::senses::*;

    /// REPL fixtures (.103, `ichiran/dict::split-pos`), 2026-05-24.
    #[test]
    fn split_pos_fixtures() {
        let cases: &[(&str, Vec<&str>)] = &[
            ("[n,adj-no]", vec!["n", "adj-no"]),
            ("[]", vec![""]),
            ("[n]", vec!["n"]),
            ("[adv,adv-to,vs]", vec!["adv", "adv-to", "vs"]),
            ("[ctr]", vec!["ctr"]),
            ("[v5u,vt]", vec!["v5u", "vt"]),
        ];
        for (pos_str, expected) in cases {
            assert_eq!(&split_pos(pos_str), expected, "pos_str={pos_str:?}");
        }
    }
}

mod get_senses_json {
    use crate::dict::dao::KanaText;
    use crate::dict::dao::KanjiText;
    use crate::dict::senses::*;
    use std::future::Ready;
    use std::sync::Arc;

    type GetterFut = Ready<Result<Option<KaniWordDispatchEnum>, sqlx::Error>>;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn json(values: &[Value]) -> String {
        serde_json::to_string(values).unwrap()
    }

    async fn kanji_reading(ctx: &KaniranContext, seq: i32, text: &str) -> KaniWordDispatchEnum {
        let row: KanjiText =
            sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 AND text = $2")
                .bind(seq)
                .bind(text)
                .fetch_one(&ctx.pool)
                .await
                .unwrap();
        KaniWordDispatchEnum::Kanji(row)
    }

    async fn kana_reading(ctx: &KaniranContext, seq: i32, text: &str) -> KaniWordDispatchEnum {
        let row: KanaText = sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = $2")
            .bind(seq)
            .bind(text)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        KaniWordDispatchEnum::Kana(row)
    }

    /// REPL fixtures (.103, `(jsown:to-json (get-senses-json …))`),
    /// 2026-05-24. No reading/getter, no pos-list: every sense is
    /// collected. Covers `field` ({food}), multi-pos `[adj-no,n]`, the
    /// `[]`-second-sense `rpos` carry-forward (1447690 → both `[n]`), and
    /// the `s_inf` → `info` path with non-ASCII text (serde emits raw
    /// UTF-8, not jsown's `\u` escapes).
    #[tokio::test]
    async fn plain_collect_all() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, &str)] = &[
            (
                1001390,
                r#"[{"pos":"[n]","gloss":"oden; dish of various ingredients, e.g. egg, daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi","field":"{food}"}]"#,
            ),
            (1577900, r#"[{"pos":"[adj-no,n]","gloss":"eternity"}]"#),
            (
                1447690,
                r#"[{"pos":"[n]","gloss":"Tokyo"},{"pos":"[n]","gloss":"Tokyo Metropolis"}]"#,
            ),
            (
                1000230,
                r#"[{"pos":"[exp]","gloss":"useless; no good; hopeless","info":"commonly used with i-adjective inflections, e.g. あかんかった, あかんくない"},{"pos":"[exp]","gloss":"cannot; must not; not allowed"}]"#,
            ),
            (
                1000320,
                r#"[{"pos":"[pn]","gloss":"there; over there; that place; yonder; you-know-where","info":"place physically distant from both speaker and listener"},{"pos":"[n]","gloss":"genitals; private parts; nether regions"},{"pos":"[n]","gloss":"that far; that much; that point","info":"something psychologically distant from both speaker and listener"}]"#,
            ),
        ];
        for (seq, expected) in cases {
            let result = get_senses_json(&ctx, *seq, &[], None, None::<GetterFut>)
                .await
                .unwrap();
            assert_eq!(json(&result), *expected, "seq={seq}");
        }
    }

    /// REPL fixtures (.103), 2026-05-24. `pos-list` filter against the
    /// carried-forward `lpos`. 1577900 keeps/drops on `n`/`xxx`; 1447690
    /// `n` keeps both senses (the `[]` sense inherits `lpos=["n"]`);
    /// 1000320 `n` drops the leading `pn` sense; 1199330 `ctr` keeps only
    /// the counter sense (mirrors the `:pos-list '("ctr")` call site).
    #[tokio::test]
    async fn pos_list_filter() {
        let ctx = ctx_from_env().await;
        struct Case {
            seq: i32,
            pos: Vec<String>,
            expected: &'static str,
        }
        let cases = [
            Case {
                seq: 1577900,
                pos: vec!["n".to_owned()],
                expected: r#"[{"pos":"[adj-no,n]","gloss":"eternity"}]"#,
            },
            Case {
                seq: 1577900,
                pos: vec!["xxx".to_owned()],
                expected: "[]",
            },
            Case {
                seq: 1447690,
                pos: vec!["n".to_owned()],
                expected: r#"[{"pos":"[n]","gloss":"Tokyo"},{"pos":"[n]","gloss":"Tokyo Metropolis"}]"#,
            },
            Case {
                seq: 1000320,
                pos: vec!["n".to_owned()],
                expected: r#"[{"pos":"[n]","gloss":"genitals; private parts; nether regions"},{"pos":"[n]","gloss":"that far; that much; that point","info":"something psychologically distant from both speaker and listener"}]"#,
            },
            Case {
                seq: 1199330,
                pos: vec!["ctr".to_owned()],
                expected: r#"[{"pos":"[ctr]","gloss":"counter for occurrences"}]"#,
            },
        ];
        for case in &cases {
            let result = get_senses_json(&ctx, case.seq, &case.pos, None, None::<GetterFut>)
                .await
                .unwrap();
            assert_eq!(
                json(&result),
                case.expected,
                "seq={} pos={:?}",
                case.seq,
                case.pos
            );
        }
    }

    /// REPL fixtures (.103), 2026-05-24. `:reading` restriction path
    /// (1339160 sense 1 has stagk 出し / stagr ダシ; 1115120 sense 1 has
    /// stagk 風太郎). A `stagk`/`stagr` member passes (出し / ダシ); a
    /// non-matching kanji is filtered (出汁 → nil; プー太郎 → nil); the
    /// restricted-reading `Found` path passes (ぷうたろう → 風太郎).
    #[tokio::test]
    async fn reading_restriction() {
        let ctx = ctx_from_env().await;
        let both_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"},{"pos":"[n]","gloss":"pretext; excuse; pretense (pretence); dupe; front man"}]"#;
        let one_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"}]"#;
        let one_taro = r#"[{"pos":"[n]","gloss":"unemployed person; vagabond; floater; vagrant"}]"#;
        let both_taro = r#"[{"pos":"[n]","gloss":"unemployed person; vagabond; floater; vagrant"},{"pos":"[n]","gloss":"day labourer (esp. on the docks)"}]"#;

        // 出汁 (kanji): sense 1 filtered (only ダシ nokanji matches → nil)
        let reading = kanji_reading(&ctx, 1339160, "出汁").await;
        let result = get_senses_json(&ctx, 1339160, &[], Some(reading), None::<GetterFut>)
            .await
            .unwrap();
        assert_eq!(json(&result), one_dashi, "1339160 出汁");
        // 出し (kanji): member of stagk → both pass
        let reading = kanji_reading(&ctx, 1339160, "出し").await;
        let result = get_senses_json(&ctx, 1339160, &[], Some(reading), None::<GetterFut>)
            .await
            .unwrap();
        assert_eq!(json(&result), both_dashi, "1339160 出し");
        // ダシ (kana): member of stagr → both pass
        let reading = kana_reading(&ctx, 1339160, "ダシ").await;
        let result = get_senses_json(&ctx, 1339160, &[], Some(reading), None::<GetterFut>)
            .await
            .unwrap();
        assert_eq!(json(&result), both_dashi, "1339160 ダシ");
        // プー太郎 (kanji): sense 1 filtered
        let reading = kanji_reading(&ctx, 1115120, "プー太郎").await;
        let result = get_senses_json(&ctx, 1115120, &[], Some(reading), None::<GetterFut>)
            .await
            .unwrap();
        assert_eq!(json(&result), one_taro, "1115120 プー太郎");
        // ぷうたろう (kana): match-kana-kanji Found(風太郎) → sense 1 passes
        let reading = kana_reading(&ctx, 1115120, "ぷうたろう").await;
        let result = get_senses_json(&ctx, 1115120, &[], Some(reading), None::<GetterFut>)
            .await
            .unwrap();
        assert_eq!(json(&result), both_taro, "1115120 ぷうたろう");
    }

    /// REPL fixtures (.103), 2026-05-24. `:reading-getter` lazy thunk.
    /// A getter yielding 出汁 filters sense 1 exactly like the eager
    /// `:reading` form; a getter yielding `nil` leaves the restricted
    /// sense in (the `(if rr … t)` fallthrough). 1011960 carries two
    /// stag-restricted senses (1 and 2): the nil getter fires once at
    /// sense 1 then sense 2 takes the `readp`-already-true / `reading`-nil
    /// path; the ぼたぼた getter fires once then sense 2 reuses the memoized
    /// `reading` (the `(or reading …)` short-circuit) — both keep all three.
    #[tokio::test]
    async fn reading_getter_path() {
        let ctx = ctx_from_env().await;
        let one_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"}]"#;
        let both_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"},{"pos":"[n]","gloss":"pretext; excuse; pretense (pretence); dupe; front man"}]"#;
        let all_bota = r#"[{"pos":"[adv,adv-to,vs]","gloss":"dripping; trickling; drop by drop; in drops"},{"pos":"[adv,adv-to,vs]","gloss":"wet and heavy (snow, clay, etc.)"},{"pos":"[adv,adv-to]","gloss":"(moving) slowly"}]"#;

        // getter → 出汁: sense 1 filtered
        let reading = kanji_reading(&ctx, 1339160, "出汁").await;
        let getter = std::future::ready(Ok(Some(reading)));
        let result = get_senses_json(&ctx, 1339160, &[], None, Some(getter))
            .await
            .unwrap();
        assert_eq!(json(&result), one_dashi, "getter 出汁");

        // getter → nil: restricted sense passes
        let getter = std::future::ready(Ok(None));
        let result = get_senses_json(&ctx, 1339160, &[], None, Some(getter))
            .await
            .unwrap();
        assert_eq!(json(&result), both_dashi, "getter nil");

        // two stag senses, nil getter: readp-true path on sense 2
        let getter = std::future::ready(Ok(None));
        let result = get_senses_json(&ctx, 1011960, &[], None, Some(getter))
            .await
            .unwrap();
        assert_eq!(json(&result), all_bota, "getter nil, two stag senses");

        // two stag senses, ぼたぼた getter (member of both): memoized reading reused
        let reading = kana_reading(&ctx, 1011960, "ぼたぼた").await;
        let getter = std::future::ready(Ok(Some(reading)));
        let result = get_senses_json(&ctx, 1011960, &[], None, Some(getter))
            .await
            .unwrap();
        assert_eq!(json(&result), all_bota, "getter ぼたぼた, two stag senses");
    }
}

mod short_sense_str {
    use crate::dict::senses::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::short-sense-str`), 2026-05-24.
    /// Covers no with-pos (first sense's glosses), with-pos matching a
    /// sense's pos, with-pos with no matching sense (→ nil), and an
    /// unknown seq (→ nil), across a noun entry (1582710 日本) and a
    /// verb entry (1358280 食べる, pos v1).
    #[tokio::test]
    async fn short_sense_str_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, Option<&str>, Option<&str>)] = &[
            (1582710, None, Some("Japan")),
            (1358280, None, Some("to eat")),
            (1358280, Some("v1"), Some("to eat")),
            (1358280, Some("n"), None),
            (1582710, Some("v1"), None),
            (1582710, Some("n"), Some("Japan")),
            (999999, None, None),
            (999999, Some("v1"), None),
        ];
        for (seq, with_pos, expected) in cases {
            assert_eq!(
                short_sense_str(&ctx, *seq, *with_pos)
                    .await
                    .unwrap()
                    .as_deref(),
                *expected,
                "seq={seq} with_pos={with_pos:?}"
            );
        }
    }
}
