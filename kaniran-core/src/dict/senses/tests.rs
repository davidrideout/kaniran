mod get_senses_raw {
    use crate::dict::senses::*;
    // Run with `--test-threads=1` (database tests).

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[test]
    fn unknown_seq_returns_empty() {
        let ctx = ctx_from_env();
        let result = get_senses_raw(&ctx, 999999).unwrap();
        assert_eq!(result, Vec::<RawSense>::new());
    }

    #[test]
    fn simple_single_sense() {
        let ctx = ctx_from_env();
        let result = get_senses_raw(&ctx, 1582710).unwrap();
        assert_eq!(
            result,
            vec![RawSense {
                ord: 0,
                gloss: "Japan".to_string(),
                props: vec![("pos".to_string(), vec!["n".to_string()])],
            }]
        );
    }

    #[test]
    fn multi_value_pos_single_sense() {
        let ctx = ctx_from_env();
        let result = get_senses_raw(&ctx, 1577900).unwrap();
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

    #[test]
    fn field_tag_present() {
        // A "field" prop ("food") sits alongside "pos" in the props list.
        let ctx = ctx_from_env();
        let result = get_senses_raw(&ctx, 1001390).unwrap();
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

    #[test]
    fn stagk_tag_and_multiple_pos() {
        // A "stagk" prop and a multi-value "pos" prop both come through.
        let ctx = ctx_from_env();
        let result = get_senses_raw(&ctx, 1000300).unwrap();
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

    #[test]
    fn final_group_bag_not_reversed_asymmetry() {
        // Two senses hold the same two stagr values but in opposite order: the
        // last sense's values come out unreversed relative to the others.
        let ctx = ctx_from_env();
        let result = get_senses_raw(&ctx, 1011960).unwrap();
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

    #[test]
    fn sense_with_no_props_yields_empty_props() {
        // A sense with no props comes back with an empty props list.
        let ctx = ctx_from_env();
        let result = get_senses_raw(&ctx, 1447690).unwrap();
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
    // Run with `--test-threads=1` (database tests).

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[test]
    fn unknown_seq_returns_empty() {
        let ctx = ctx_from_env();
        let result = get_senses(&ctx, 999999).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn simple_single_sense() {
        let ctx = ctx_from_env();
        let result = get_senses(&ctx, 1582710).unwrap();
        assert_eq!(
            result,
            vec![(
                "[n]".to_string(),
                "Japan".to_string(),
                vec![("pos".to_string(), vec!["n".to_string()])],
            )]
        );
    }

    #[test]
    fn multi_value_pos() {
        let ctx = ctx_from_env();
        let result = get_senses(&ctx, 1577900).unwrap();
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

    #[test]
    fn field_tag_preserved_in_props() {
        let ctx = ctx_from_env();
        let result = get_senses(&ctx, 1001390).unwrap();
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

    #[test]
    fn second_sense_no_pos_yields_empty_brackets() {
        let ctx = ctx_from_env();
        let result = get_senses(&ctx, 1447690).unwrap();
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
    // Run with `--test-threads=1` (database tests).

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[test]
    fn unknown_seq_yields_empty_string() {
        let ctx = ctx_from_env();
        let result = get_senses_str(&ctx, 999999).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn simple_single_sense() {
        let ctx = ctx_from_env();
        let result = get_senses_str(&ctx, 1582710).unwrap();
        assert_eq!(result, "1. [n] Japan");
    }

    #[test]
    fn multi_value_pos() {
        let ctx = ctx_from_env();
        let result = get_senses_str(&ctx, 1577900).unwrap();
        assert_eq!(result, "1. [adj-no,n] eternity");
    }

    #[test]
    fn field_braced_before_gloss() {
        let ctx = ctx_from_env();
        let result = get_senses_str(&ctx, 1001390).unwrap();
        assert_eq!(
            result,
            "1. [n] {food} oden; dish of various ingredients, e.g. egg, daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi"
        );
    }

    #[test]
    fn multi_field_joined_by_comma() {
        let ctx = ctx_from_env();
        let result = get_senses_str(&ctx, 1014100).unwrap();
        assert_eq!(result, "1. [n] {physics,chem} isotope");
    }

    #[test]
    fn s_inf_in_double_angle_brackets() {
        let ctx = ctx_from_env();
        let result = get_senses_str(&ctx, 900000).unwrap();
        assert_eq!(
            result,
            "1. [suf] 《after the -masu stem of a verb》 to seem to want to (do something)"
        );
    }

    #[test]
    fn field_and_s_inf_both_present() {
        let ctx = ctx_from_env();
        let result = get_senses_str(&ctx, 1005660).unwrap();
        assert_eq!(
            result,
            "1. [n] {food} 《from the sound of the dish being prepared》 shabu-shabu; hot pot dish where thinly sliced meat is boiled quickly and then dipped in sauce"
        );
    }

    #[test]
    fn multi_sense_separated_by_newline_no_trailing() {
        // Sense 2 has an empty pos, so it inherits sense 1's "[n]".
        let ctx = ctx_from_env();
        let result = get_senses_str(&ctx, 1447690).unwrap();
        assert_eq!(result, "1. [n] Tokyo\n2. [n] Tokyo Metropolis");
    }

    #[test]
    fn three_senses_mixed_props() {
        let ctx = ctx_from_env();
        let result = get_senses_str(&ctx, 1011960).unwrap();
        assert_eq!(
            result,
            "1. [adv,adv-to,vs] dripping; trickling; drop by drop; in drops\n2. [adv,adv-to,vs] wet and heavy (snow, clay, etc.)\n3. [adv,adv-to] (moving) slowly"
        );
    }

    #[test]
    fn five_senses_with_s_inf_subset() {
        let ctx = ctx_from_env();
        let result = get_senses_str(&ctx, 1000090).unwrap();
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

    /// Matches a kana reading against a kanji form under a restriction list.
    /// Readings used: だし (nokanji false), ダシ (nokanji true), and the
    /// 出し / 出汁 kanji forms.
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

    fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn props_of(ctx: &KaniranContext, seq: i32, ord: i32) -> Vec<(String, Vec<String>)> {
        get_senses_raw(ctx, seq)
            
            .unwrap()
            .into_iter()
            .find(|s| s.ord == ord)
            .expect("sense ord present")
            .props
    }

    fn reading_of(
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
                    
                    .unwrap();
            KaniWordDispatchEnum::Kanji(row)
        } else {
            let row: KanaText =
                sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = $2")
                    .bind(seq)
                    .bind(text)
                    .fetch_one(&ctx.pool)
                    
                    .unwrap();
            KaniWordDispatchEnum::Kana(row)
        }
    }

    /// Whether a reading passes a sense's reading restrictions. Covers: no
    /// restriction (everything passes); a reading directly listed as allowed;
    /// a kanji-only restriction rejecting a kana reading and vice versa; a
    /// restriction list resolved against the database to accept or reject a
    /// kanji; and the case where a kana reading resolves to a specific
    /// matched kanji string.
    #[test]
    fn match_sense_restrictions_fixtures() {
        let ctx = ctx_from_env();

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
            // No restriction: every reading passes.
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
            // Restricted to kanji 出し and kana ダシ.
            Case {
                seq: 1339160,
                ord: 1,
                text: "出し",
                kanji: true,
                expected: yes(),
            }, // listed kanji
            Case {
                seq: 1339160,
                ord: 1,
                text: "ダシ",
                kanji: false,
                expected: yes(),
            }, // listed kana
            Case {
                seq: 1339160,
                ord: 1,
                text: "出汁",
                kanji: true,
                expected: None,
            }, // kanji not listed
            Case {
                seq: 1339160,
                ord: 1,
                text: "だし",
                kanji: false,
                expected: yes(),
            }, // kana, no kana restriction
            // Restricted to kanji only (遇う).
            Case {
                seq: 1000300,
                ord: 0,
                text: "遇う",
                kanji: true,
                expected: yes(),
            }, // listed kanji
            Case {
                seq: 1000300,
                ord: 0,
                text: "配う",
                kanji: true,
                expected: None,
            }, // kanji not listed
            Case {
                seq: 1000300,
                ord: 0,
                text: "あしらう",
                kanji: false,
                expected: yes(),
            }, // kana passes when only kanji is restricted
            // Restricted to kana only (ボタボタ / ぼたぼた).
            Case {
                seq: 1011960,
                ord: 1,
                text: "ボタボタ",
                kanji: false,
                expected: yes(),
            }, // listed kana
            Case {
                seq: 1011960,
                ord: 1,
                text: "ポタポタ",
                kanji: false,
                expected: None,
            }, // kana not listed
            // Kanji resolved against the database accepts the reading.
            Case {
                seq: 1580140,
                ord: 0,
                text: "出端",
                kanji: true,
                expected: yes(),
            },
            // Restricted to a kanji whose readings resolve to a kanji string.
            Case {
                seq: 1115120,
                ord: 1,
                text: "風太郎",
                kanji: true,
                expected: yes(),
            }, // listed kanji
            Case {
                seq: 1115120,
                ord: 1,
                text: "プー太郎",
                kanji: true,
                expected: None,
            }, // kanji not listed
            Case {
                seq: 1115120,
                ord: 1,
                text: "プータロー",
                kanji: false,
                expected: None,
            }, // nokanji kana never matches a kanji
            Case {
                seq: 1115120,
                ord: 1,
                text: "ぷうたろう",
                kanji: false,
                expected: found("風太郎"),
            }, // kana resolves to the matched kanji
            Case {
                seq: 1115120,
                ord: 1,
                text: "ふうたろう",
                kanji: false,
                expected: found("風太郎"),
            }, // kana resolves to the matched kanji
            Case {
                seq: 1115120,
                ord: 1,
                text: "プーたろう",
                kanji: false,
                expected: None,
            }, // kana whose kanji is absent from the restriction
        ];

        for case in &cases {
            let props = props_of(&ctx, case.seq, case.ord);
            let reading = reading_of(&ctx, case.seq, case.text, case.kanji);
            let actual = match_sense_restrictions(&ctx, case.seq, &props, &reading)
                
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

    type GetterFut = Ready<Result<Option<KaniWordDispatchEnum>, crate::conn::KaniDbError>>;

    fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn json(values: &[Value]) -> String {
        serde_json::to_string(values).unwrap()
    }

    fn kanji_reading(ctx: &KaniranContext, seq: i32, text: &str) -> KaniWordDispatchEnum {
        let row: KanjiText =
            sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 AND text = $2")
                .bind(seq)
                .bind(text)
                .fetch_one(&ctx.pool)
                
                .unwrap();
        KaniWordDispatchEnum::Kanji(row)
    }

    fn kana_reading(ctx: &KaniranContext, seq: i32, text: &str) -> KaniWordDispatchEnum {
        let row: KanaText = sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = $2")
            .bind(seq)
            .bind(text)
            .fetch_one(&ctx.pool)
            
            .unwrap();
        KaniWordDispatchEnum::Kana(row)
    }

    /// With no reading and no pos filter, every sense is collected. Covers a
    /// field ({food}), a multi-pos bracket ([adj-no,n]), a second sense with
    /// empty pos inheriting the first sense's pos, and the s_inf note rendered
    /// into an `info` field with non-ASCII text.
    #[test]
    fn plain_collect_all() {
        let ctx = ctx_from_env();
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
                
                .unwrap();
            assert_eq!(json(&result), *expected, "seq={seq}");
        }
    }

    /// The pos filter keeps only senses whose carried-forward pos matches.
    /// A matching pos keeps the sense, a non-matching one drops it, a sense
    /// with empty pos inherits and matches the prior sense's pos, and a
    /// `ctr` filter keeps only the counter sense.
    #[test]
    fn pos_list_filter() {
        let ctx = ctx_from_env();
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

    /// When a reading is supplied, senses are filtered by their reading
    /// restrictions: a listed kanji or kana passes, a non-listed kanji is
    /// dropped, and a kana that resolves to a listed kanji passes.
    #[test]
    fn reading_restriction() {
        let ctx = ctx_from_env();
        let both_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"},{"pos":"[n]","gloss":"pretext; excuse; pretense (pretence); dupe; front man"}]"#;
        let one_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"}]"#;
        let one_taro = r#"[{"pos":"[n]","gloss":"unemployed person; vagabond; floater; vagrant"}]"#;
        let both_taro = r#"[{"pos":"[n]","gloss":"unemployed person; vagabond; floater; vagrant"},{"pos":"[n]","gloss":"day labourer (esp. on the docks)"}]"#;

        // 出汁 (kanji): not in the restriction, so sense 1 is filtered out.
        let reading = kanji_reading(&ctx, 1339160, "出汁");
        let result = get_senses_json(&ctx, 1339160, &[], Some(reading), None::<GetterFut>)
            
            .unwrap();
        assert_eq!(json(&result), one_dashi, "1339160 出汁");
        // 出し (kanji): listed in the restriction, so both senses pass.
        let reading = kanji_reading(&ctx, 1339160, "出し");
        let result = get_senses_json(&ctx, 1339160, &[], Some(reading), None::<GetterFut>)
            
            .unwrap();
        assert_eq!(json(&result), both_dashi, "1339160 出し");
        // ダシ (kana): listed in the restriction, so both senses pass.
        let reading = kana_reading(&ctx, 1339160, "ダシ");
        let result = get_senses_json(&ctx, 1339160, &[], Some(reading), None::<GetterFut>)
            
            .unwrap();
        assert_eq!(json(&result), both_dashi, "1339160 ダシ");
        // プー太郎 (kanji): not in the restriction, so sense 1 is filtered out.
        let reading = kanji_reading(&ctx, 1115120, "プー太郎");
        let result = get_senses_json(&ctx, 1115120, &[], Some(reading), None::<GetterFut>)
            
            .unwrap();
        assert_eq!(json(&result), one_taro, "1115120 プー太郎");
        // ぷうたろう (kana): resolves to the listed kanji 風太郎, so sense 1 passes.
        let reading = kana_reading(&ctx, 1115120, "ぷうたろう");
        let result = get_senses_json(&ctx, 1115120, &[], Some(reading), None::<GetterFut>)
            
            .unwrap();
        assert_eq!(json(&result), both_taro, "1115120 ぷうたろう");
    }

    /// A reading supplied lazily through a getter behaves like an eager
    /// reading: a getter yielding 出汁 filters the restricted sense out, and a
    /// getter yielding nothing leaves it in. The getter fires only once even
    /// across multiple restricted senses; the resolved reading is reused.
    #[test]
    fn reading_getter_path() {
        let ctx = ctx_from_env();
        let one_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"}]"#;
        let both_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"},{"pos":"[n]","gloss":"pretext; excuse; pretense (pretence); dupe; front man"}]"#;
        let all_bota = r#"[{"pos":"[adv,adv-to,vs]","gloss":"dripping; trickling; drop by drop; in drops"},{"pos":"[adv,adv-to,vs]","gloss":"wet and heavy (snow, clay, etc.)"},{"pos":"[adv,adv-to]","gloss":"(moving) slowly"}]"#;

        // Getter yields 出汁: sense 1 is filtered out.
        let reading = kanji_reading(&ctx, 1339160, "出汁");
        let getter = std::future::ready(Ok(Some(reading)));
        let result = get_senses_json(&ctx, 1339160, &[], None, Some(getter))
            
            .unwrap();
        assert_eq!(json(&result), one_dashi, "getter 出汁");

        // Getter yields nothing: the restricted sense passes.
        let getter = std::future::ready(Ok(None));
        let result = get_senses_json(&ctx, 1339160, &[], None, Some(getter))
            
            .unwrap();
        assert_eq!(json(&result), both_dashi, "getter nil");

        // Two restricted senses, getter yields nothing: both senses kept.
        let getter = std::future::ready(Ok(None));
        let result = get_senses_json(&ctx, 1011960, &[], None, Some(getter))
            
            .unwrap();
        assert_eq!(json(&result), all_bota, "getter nil, two stag senses");

        // Two restricted senses, getter yields ぼたぼた (listed in both):
        // the resolved reading is reused, all three senses kept.
        let reading = kana_reading(&ctx, 1011960, "ぼたぼた");
        let getter = std::future::ready(Ok(Some(reading)));
        let result = get_senses_json(&ctx, 1011960, &[], None, Some(getter))
            
            .unwrap();
        assert_eq!(json(&result), all_bota, "getter ぼたぼた, two stag senses");
    }
}

mod short_sense_str {
    use crate::dict::senses::*;

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// With no pos given, returns the first sense's gloss. With a pos, returns
    /// the matching sense's gloss, or nothing when no sense matches. An unknown
    /// sequence returns nothing.
    #[test]
    fn short_sense_str_fixtures() {
        let ctx = ctx_from_env();
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
                    
                    .unwrap()
                    .as_deref(),
                *expected,
                "seq={seq} with_pos={with_pos:?}"
            );
        }
    }
}
