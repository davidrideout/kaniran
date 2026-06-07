mod select_conjs {
    use crate::dict::conj::*;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::select-conjs`), 2026-05-24.
    /// - 2028980: single via-null conjugation (id 2343254, from 2089020) —
    ///   mirrors `tests.lisp:651`.
    /// - 1156880: via-null branch returns two rows (366552, 661748); the
    ///   seq's via-not-null row (705712) is excluded.
    /// - 1257260: no via-null rows, so the `or` falls back to all rows
    ///   (1239109, 1239126), both via-not-null.
    #[tokio::test]
    async fn select_conjs_nil_conj_ids() {
        let ctx = ctx_from_env().await;

        let r2028980 = select_conjs(&ctx, 2028980, None).await.unwrap();
        let mut ids: Vec<i32> = r2028980.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![2343254]);
        assert_eq!(r2028980[0].seq_from, 2089020);
        assert_eq!(r2028980[0].seq_via, None);

        let r1156880 = select_conjs(&ctx, 1156880, None).await.unwrap();
        let mut ids: Vec<i32> = r1156880.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![366552, 661748]);
        assert!(r1156880.iter().all(|c| c.seq_via.is_none()));

        // or-fallback: no via-null rows → all rows (both via-not-null).
        let r1257260 = select_conjs(&ctx, 1257260, None).await.unwrap();
        let mut ids: Vec<i32> = r1257260.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![1239109, 1239126]);
        assert!(r1257260.iter().all(|c| c.seq_via.is_some()));
    }

    /// REPL: `(select-conjs 2028980 :root)` → `NIL`.
    #[tokio::test]
    async fn select_conjs_root_is_empty() {
        let ctx = ctx_from_env().await;
        let result = select_conjs(&ctx, 2028980, Some(&WordConjugations::Root))
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    /// REPL: `(select-conjs 1156880 (list 366552))` → only the requested
    /// id, regardless of the via-null preference (no `or` fallback). The
    /// via-not-null row (705712) is reachable through an explicit id list.
    #[tokio::test]
    async fn select_conjs_explicit_ids() {
        let ctx = ctx_from_env().await;

        let one = select_conjs(&ctx, 1156880, Some(&WordConjugations::Ids(vec![366552])))
            .await
            .unwrap();
        let ids: Vec<i32> = one.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![366552]);

        // The via-not-null row is selectable by id even though the
        // nil-conj-ids path filters it out.
        let via_row = select_conjs(&ctx, 1156880, Some(&WordConjugations::Ids(vec![705712])))
            .await
            .unwrap();
        assert_eq!(via_row.len(), 1);
        assert_eq!(via_row[0].seq_via, Some(1156890));

        // ids that don't belong to the seq are filtered by the `seq =` clause.
        let none = select_conjs(&ctx, 1156880, Some(&WordConjugations::Ids(vec![1])))
            .await
            .unwrap();
        assert!(none.is_empty());
    }
}

mod conj_type_order {
    use crate::dict::conj::*;

    /// REPL fixtures (.103, `ichiran/dict::conj-type-order`), 2026-05-24.
    /// Covers the 10↔13 swap and the identity fall-through.
    #[test]
    fn conj_type_order_fixtures() {
        let cases: &[(i32, i32)] = &[(10, 13), (13, 10), (1, 1), (0, 0), (99, 99)];
        for (conj_type, expected) in cases {
            assert_eq!(
                conj_type_order(*conj_type),
                *expected,
                "conj_type={conj_type}"
            );
        }
    }
}

mod is_rareru {
    use crate::dict::conj::*;

    /// REPL fixtures (.103, `ichiran/dict::is-rareru`), 2026-05-24.
    /// Covers each of the four suffixes, a non-rareru form, the empty
    /// string, a rareru substring not at the end (`られるよ` → false), and
    /// a kana-only suffix.
    #[test]
    fn is_rareru_fixtures() {
        let cases: &[(&str, bool)] = &[
            ("食べられる", true),
            ("食べられます", true),
            ("食べられない", true),
            ("食べられません", true),
            ("食べる", false),
            ("", false),
            ("られるよ", false),
            ("られる", true),
        ];
        for (text, expected) in cases {
            assert_eq!(is_rareru(text), *expected, "text={text:?}");
        }
    }
}

mod filter_props {
    use crate::dict::conj::*;

    fn prop(conj_id: i32, conj_type: i32, pos: &str) -> ConjProp {
        ConjProp {
            id: conj_id,
            conj_id,
            conj_type,
            pos: pos.to_string(),
            neg: None,
            fml: None,
        }
    }

    fn ids(props: &[&ConjProp]) -> Vec<i32> {
        props.iter().map(|prop| prop.conj_id).collect()
    }

    /// REPL fixtures (.103, `ichiran/dict::filter-props`), 2026-05-24.
    /// `props` = passive v1 (1), passive v5r (2, pos out of set), plain v1
    /// (3, conj-type ≠ 6), passive v1s (4), passive vk (5). Each row drops
    /// the passive v1/v1s/vk props (1,4,5) only when text is non-nil and
    /// not a rareru form. Covers nil, single string (rareru / non-rareru /
    /// empty-but-truthy), and list (with / without a rareru member /
    /// empty-list-is-nil).
    #[test]
    fn filter_props_fixtures() {
        let props = vec![
            prop(1, 6, "v1"),
            prop(2, 6, "v5r"),
            prop(3, 1, "v1"),
            prop(4, 6, "v1s"),
            prop(5, 6, "vk"),
        ];
        let some = ["食べる", "見られる"];
        let none = ["食べる", "飲む"];
        let cases: &[(FilterPropsText, Vec<i32>)] = &[
            (FilterPropsText::None, vec![1, 2, 3, 4, 5]),
            (FilterPropsText::One("食べる"), vec![2, 3]),
            (FilterPropsText::One("食べられる"), vec![1, 2, 3, 4, 5]),
            (FilterPropsText::One(""), vec![2, 3]),
            (FilterPropsText::Many(&none), vec![2, 3]),
            (FilterPropsText::Many(&some), vec![1, 2, 3, 4, 5]),
            (FilterPropsText::Many(&[]), vec![1, 2, 3, 4, 5]),
        ];
        for (text, expected) in cases {
            assert_eq!(ids(&filter_props(&props, *text)), *expected);
        }
        // empty props → empty result
        assert!(filter_props(&[], FilterPropsText::One("食べる")).is_empty());
    }
}

mod select_conjs_and_props {
    use crate::dict::conj::*;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    type FpropRow = (i32, i32, i32, String, Option<bool>, Option<bool>);
    type ConjRow = (i32, i32, i32, Option<i32>, [i32; 2], Vec<FpropRow>);

    fn project(rows: &[(Conjugation, Vec<ConjProp>, [i32; 2])]) -> Vec<ConjRow> {
        rows.iter()
            .map(|(conj, fprops, key)| {
                (
                    conj.id,
                    conj.seq,
                    conj.seq_from,
                    conj.seq_via,
                    *key,
                    fprops
                        .iter()
                        .map(|p| (p.id, p.conj_id, p.conj_type, p.pos.clone(), p.neg, p.fml))
                        .collect(),
                )
            })
            .collect()
    }

    /// REPL: `(select-conjs-and-props 1156880)` → two via-null
    /// conjugations sorted by `(0 val)`. conj 661748 has prop type 13
    /// → `conj-type-order` 10 → key `(0 10)`, sorts ahead of conj
    /// 366552 (prop type 10 → `conj-type-order` 13 → key `(0 13)`),
    /// reordering the `select-conjs` input. Exercises the val swap and
    /// the sort, with nil text (all props kept).
    #[tokio::test]
    async fn via_null_sorted_by_val() {
        let ctx = ctx_from_env().await;
        let rows = select_conjs_and_props(&ctx, 1156880, None, FilterPropsText::None)
            .await
            .unwrap();
        let expected: Vec<ConjRow> = vec![
            (
                661748,
                1156880,
                1156890,
                None,
                [0, 10],
                vec![(676835, 661748, 13, "v1".to_string(), None, None)],
            ),
            (
                366552,
                1156880,
                1156870,
                None,
                [0, 13],
                vec![(
                    374822,
                    366552,
                    10,
                    "v5m".to_string(),
                    Some(false),
                    Some(false),
                )],
            ),
        ];
        assert_eq!(project(&rows), expected);
    }

    /// REPL: `(select-conjs-and-props 1257260)` → no via-null rows, so
    /// `select-conjs` falls back to all rows; both have non-null via →
    /// key first element 1. Sorted `(1 10)` before `(1 13)`. Exercises
    /// the via-flag=1 branch and the or-fallback path.
    #[tokio::test]
    async fn via_not_null_flag_one() {
        let ctx = ctx_from_env().await;
        let rows = select_conjs_and_props(&ctx, 1257260, None, FilterPropsText::None)
            .await
            .unwrap();
        let expected: Vec<ConjRow> = vec![
            (
                1239109,
                1257260,
                1609260,
                Some(10036077),
                [1, 10],
                vec![(1254564, 1239109, 13, "v1".to_string(), None, None)],
            ),
            (
                1239126,
                1257260,
                1609260,
                Some(10036081),
                [1, 13],
                vec![(
                    1254581,
                    1239126,
                    10,
                    "v5s".to_string(),
                    Some(false),
                    Some(false),
                )],
            ),
        ];
        assert_eq!(project(&rows), expected);
    }

    /// REPL fixtures (.103, `ichiran/dict::select-conjs-and-props`),
    /// 2026-05-24. seq 1232500 has one via-null conjugation (159588)
    /// with a passive prop (type 6, pos v1). The key stays `(0 6)` and
    /// `val` stays 6 across every text — `val` reads the *unfiltered*
    /// props — while `fprops` drops the passive prop exactly when
    /// `filter-props` would: text non-nil, not a rareru form. Covers
    /// nil / single rareru / single non-rareru / list with a rareru /
    /// list without a rareru.
    #[tokio::test]
    async fn text_threads_to_filter_props() {
        let ctx = ctx_from_env().await;
        let prop = (
            163127,
            159588,
            6,
            "v1".to_string(),
            Some(false),
            Some(false),
        );
        let kept: Vec<ConjRow> = vec![(159588, 1232500, 2864818, None, [0, 6], vec![prop.clone()])];
        let dropped: Vec<ConjRow> = vec![(159588, 1232500, 2864818, None, [0, 6], vec![])];

        let rareru = ["食べる", "見られる"];
        let no_rareru = ["食べる", "飲む"];
        let cases: &[(FilterPropsText, &Vec<ConjRow>)] = &[
            (FilterPropsText::None, &kept),
            (FilterPropsText::One("見られる"), &kept),
            (FilterPropsText::One("食べる"), &dropped),
            (FilterPropsText::Many(&rareru), &kept),
            (FilterPropsText::Many(&no_rareru), &dropped),
        ];
        for (text, expected) in cases {
            let rows = select_conjs_and_props(&ctx, 1232500, None, *text)
                .await
                .unwrap();
            assert_eq!(&project(&rows), *expected, "text variant mismatch");
        }
    }

    /// REPL: `(select-conjs-and-props 2028980 :root)` → `NIL`
    /// (`select-conjs … :root` returns no conjugations).
    #[tokio::test]
    async fn root_conj_ids_empty() {
        let ctx = ctx_from_env().await;
        let rows = select_conjs_and_props(
            &ctx,
            2028980,
            Some(&WordConjugations::Root),
            FilterPropsText::None,
        )
        .await
        .unwrap();
        assert!(rows.is_empty());
    }
}

mod print_conj_info {
    use crate::dict::conj::*;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn render(
        ctx: &KaniranContext,
        seq: i32,
        conjugations: Option<&WordConjugations>,
    ) -> String {
        let mut out = String::new();
        print_conj_info(ctx, seq, conjugations, &mut out)
            .await
            .unwrap();
        out
    }

    /// REPL fixtures (.103, `ichiran/dict::print-conj-info` via
    /// `(with-output-to-string (s) (print-conj-info seq :out s))`),
    /// 2026-05-24, `conjugations` = nil. Covers:
    /// - 1156880: two via-null conjugations, one prop each — the
    ///   entry-info-short branch repeated, each prop opens with "[".
    /// - 1184270: one via-null conjugation with two props — the
    ///   `first` toggle ("[" then " ") inside one " ]".
    /// - 1257260: two non-null via conjugations — the " --(via)--"
    ///   branch with a recursive call producing the via entry.
    /// - 10674648: two conjugations sharing via 10327845 — the second
    ///   is dropped by `(member via via-used)`, so only one block prints.
    /// - 1358280: no conjugations (root) → empty output.
    #[tokio::test]
    async fn print_conj_info_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, &str)] = &[
            (
                1156880,
                "\n[ Conjugation: [v1] Continuative (~i)\n  慰める 【なぐさめる】 : to comfort; to console; to amuse ]\n[ Conjugation: [v5m] Imperative Affirmative Plain\n  慰む 【なぐさむ】 : to feel comforted; to be in good spirits; to feel better; to forget one's worries ]",
            ),
            (
                1184270,
                "\n[ Conjugation: [v5aru] Imperative Affirmative Plain\n  Conjugation: [v5aru] Continuative (~i)\n  下さる 【くださる】 : to give; to confer; to bestow ]",
            ),
            (
                1257260,
                "\n[ Conjugation: [v1] Continuative (~i)\n --(via)--\n[ Conjugation: [v5r] Causative Affirmative Plain\n  嫌がる 【いやがる】 : to appear uncomfortable (with); to seem to hate; to express dislike ] ]\n[ Conjugation: [v5s] Imperative Affirmative Plain\n --(via)--\n[ Conjugation: [v5r] Causative (~su) Affirmative Plain\n  嫌がる 【いやがる】 : to appear uncomfortable (with); to seem to hate; to express dislike ] ]",
            ),
            (
                10674648,
                "\n[ Conjugation: [v1] Past (~ta) Affirmative Plain\n --(via)--\n[ Conjugation: [v5s] Potential Affirmative Plain\n  くねらす : to wriggle; to twist (one's body); to writhe ]\n[ Conjugation: [v5r] Causative Affirmative Plain\n  くねる : to bend loosely back and forth; to wriggle; to be crooked ] ]",
            ),
            (1358280, ""),
        ];
        for (seq, expected) in cases {
            assert_eq!(&render(&ctx, *seq, None).await, expected, "seq={seq}");
        }
    }

    /// REPL fixtures (.103, `print-conj-info 1156880` with
    /// `:conjugations`), 2026-05-24. `:root` selects no conjugations
    /// (empty output); an explicit id list narrows to that single
    /// conjugation.
    #[tokio::test]
    async fn print_conj_info_conjugations_arg() {
        let ctx = ctx_from_env().await;
        assert_eq!(
            render(&ctx, 1156880, Some(&WordConjugations::Root)).await,
            "",
            "conjugations=:root"
        );
        assert_eq!(
            render(&ctx, 1156880, Some(&WordConjugations::Ids(vec![366552]))).await,
            "\n[ Conjugation: [v5m] Imperative Affirmative Plain\n  慰む 【なぐさむ】 : to feel comforted; to be in good spirits; to feel better; to forget one's worries ]",
            "conjugations=(366552)"
        );
    }
}

mod conj_info_json_star_ {
    use crate::dict::conj::*;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn json(values: &[Value]) -> String {
        serde_json::to_string(values).unwrap()
    }

    /// REPL fixtures (.103, `(jsown:to-json (conj-info-json* …))`),
    /// 2026-05-24. seq 10175587 is the past-tense (~ta) conjugation of
    /// 1370080 (尽き果てる), via-null. The kana / kanji surface both resolve
    /// the original reading (readok true); has-gloss true keeps the entry
    /// because the reading resolves; a non-matching surface and nil text
    /// leave the original reading nil (`readok` `[]`, `reading` from
    /// `reading-str-seq` of seq-from). With has-gloss true AND no resolved
    /// reading, `(return-from outer nil)` drops the entry entirely (`[]`).
    #[tokio::test]
    async fn via_null_paths() {
        let ctx = ctx_from_env().await;
        let found = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"reading":"尽き果てる 【つきはてる】","gloss":[{"pos":"[vi,v1]","gloss":"to be exhausted"}],"readok":true}]"#;
        let unresolved = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"reading":"尽き果てる 【つきはてる】","gloss":[{"pos":"[vi,v1]","gloss":"to be exhausted"}],"readok":[]}]"#;
        let dropped = "[]";

        struct Case {
            label: &'static str,
            text: FilterPropsText<'static>,
            has_gloss: bool,
            expected: &'static str,
        }
        let cases = [
            Case {
                label: "kana surface",
                text: FilterPropsText::One("つきはてた"),
                has_gloss: false,
                expected: found,
            },
            Case {
                label: "kanji surface",
                text: FilterPropsText::One("尽き果てた"),
                has_gloss: false,
                expected: found,
            },
            Case {
                label: "has-gloss, resolved",
                text: FilterPropsText::One("つきはてた"),
                has_gloss: true,
                expected: found,
            },
            Case {
                label: "non-matching surface",
                text: FilterPropsText::One("存在しない"),
                has_gloss: false,
                expected: unresolved,
            },
            Case {
                label: "nil text",
                text: FilterPropsText::None,
                has_gloss: false,
                expected: unresolved,
            },
            Case {
                label: "has-gloss, non-matching → dropped",
                text: FilterPropsText::One("存在しない"),
                has_gloss: true,
                expected: dropped,
            },
            Case {
                label: "has-gloss, nil text → dropped",
                text: FilterPropsText::None,
                has_gloss: true,
                expected: dropped,
            },
        ];
        for case in &cases {
            let result = conj_info_json_star_(&ctx, 10175587, None, case.text, case.has_gloss)
                .await
                .unwrap();
            assert_eq!(json(&result), case.expected, "case={}", case.label);
        }
    }

    /// REPL: `(conj-info-json* 10670519 :conjugations nil :text "あくどくさせた"
    /// :has-gloss …)`. seq 10670519 is the causative-past of 1000260
    /// (悪どい) via 10155281 (causative), exercising the via-not-null
    /// recursion: the entry nests the via's own conj-info-json under
    /// `"via"` and copies its `readok`.
    #[tokio::test]
    async fn via_not_null_recursion() {
        let ctx = ctx_from_env().await;
        let expected = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"via":[{"prop":[{"pos":"adj-i","type":"Causative"}],"reading":"悪どい 【あくどい】","gloss":[{"pos":"[adj-i]","gloss":"gaudy; showy; garish; loud"},{"pos":"[adj-i]","gloss":"crooked; vicious; wicked; nasty; unscrupulous; dishonest"}],"readok":true}],"readok":true}]"#;
        for has_gloss in [false, true] {
            let result = conj_info_json_star_(
                &ctx,
                10670519,
                None,
                FilterPropsText::One("あくどくさせた"),
                has_gloss,
            )
            .await
            .unwrap();
            assert_eq!(json(&result), expected, "has_gloss={has_gloss}");
        }
    }

    /// REPL: `(conj-info-json* 1156880 …)`. seq 1156880 (慰め) carries two
    /// via-null conjugations — 慰める (v1 continuative) and 慰む (v5m
    /// imperative) — so the loop emits two entries in `select-conjs-and-props`
    /// order. nil text leaves both readings unresolved (`readok` `[]`);
    /// restricting `conjugations` to one conj id (661748) emits a single
    /// entry.
    #[tokio::test]
    async fn multi_entry_and_conj_ids() {
        let ctx = ctx_from_env().await;
        let both = r#"[{"prop":[{"pos":"v1","type":"Continuative (~i)"}],"reading":"慰める 【なぐさめる】","gloss":[{"pos":"[vt,v1]","gloss":"to comfort; to console; to amuse"}],"readok":true},{"prop":[{"pos":"v5m","type":"Imperative"}],"reading":"慰む 【なぐさむ】","gloss":[{"pos":"[v5m,vi]","gloss":"to feel comforted; to be in good spirits; to feel better; to forget one's worries"},{"pos":"[vt,v5m]","gloss":"to trifle with; to fool around with"}],"readok":true}]"#;
        let both_unresolved = r#"[{"prop":[{"pos":"v1","type":"Continuative (~i)"}],"reading":"慰める 【なぐさめる】","gloss":[{"pos":"[vt,v1]","gloss":"to comfort; to console; to amuse"}],"readok":[]},{"prop":[{"pos":"v5m","type":"Imperative"}],"reading":"慰む 【なぐさむ】","gloss":[{"pos":"[v5m,vi]","gloss":"to feel comforted; to be in good spirits; to feel better; to forget one's worries"},{"pos":"[vt,v5m]","gloss":"to trifle with; to fool around with"}],"readok":[]}]"#;
        let only_one = r#"[{"prop":[{"pos":"v1","type":"Continuative (~i)"}],"reading":"慰める 【なぐさめる】","gloss":[{"pos":"[vt,v1]","gloss":"to comfort; to console; to amuse"}],"readok":true}]"#;

        let result = conj_info_json_star_(&ctx, 1156880, None, FilterPropsText::One("慰め"), false)
            .await
            .unwrap();
        assert_eq!(json(&result), both, "慰め");

        let result = conj_info_json_star_(&ctx, 1156880, None, FilterPropsText::None, false)
            .await
            .unwrap();
        assert_eq!(json(&result), both_unresolved, "nil text");

        let ids = WordConjugations::Ids(vec![661748]);
        let result = conj_info_json_star_(
            &ctx,
            1156880,
            Some(&ids),
            FilterPropsText::One("慰め"),
            false,
        )
        .await
        .unwrap();
        assert_eq!(json(&result), only_one, "conj-ids 661748");
    }
}

mod conj_info_json {
    use crate::dict::conj::*;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn json(values: &[Value]) -> String {
        serde_json::to_string(values).unwrap()
    }

    /// REPL fixtures (.103, `(jsown:to-json (conj-info-json …))`), 2026-05-24.
    /// seq 10175587 (尽き果てる, ~ta) resolves the original reading with a
    /// matching surface (readok true → kept). With nil text every entry's
    /// readok is `[]`, so `remove-if-not` empties the filtered list and the
    /// `(or fcij cij)` fallback returns the unfiltered list unchanged.
    #[tokio::test]
    async fn readok_filter_and_fallback() {
        let ctx = ctx_from_env().await;
        let found = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"reading":"尽き果てる 【つきはてる】","gloss":[{"pos":"[vi,v1]","gloss":"to be exhausted"}],"readok":true}]"#;
        let unresolved = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"reading":"尽き果てる 【つきはてる】","gloss":[{"pos":"[vi,v1]","gloss":"to be exhausted"}],"readok":[]}]"#;

        let result = conj_info_json(
            &ctx,
            10175587,
            None,
            FilterPropsText::One("つきはてた"),
            false,
        )
        .await
        .unwrap();
        assert_eq!(json(&result), found, "resolved surface");

        let result = conj_info_json(&ctx, 10175587, None, FilterPropsText::None, false)
            .await
            .unwrap();
        assert_eq!(json(&result), unresolved, "nil text → fallback to cij");

        // has-gloss + unresolved reading drops the only entry in conj-info-json*,
        // so cij is empty and (or fcij cij) is the empty list.
        let result = conj_info_json(
            &ctx,
            10175587,
            None,
            FilterPropsText::One("存在しない"),
            true,
        )
        .await
        .unwrap();
        assert_eq!(json(&result), "[]", "has-gloss drop → empty");
    }

    /// REPL: `(conj-info-json 10670519 :conjugations nil :text "あくどくさせた"
    /// :has-gloss nil)`. The via-not-null entry keeps its recursive `via`
    /// payload (readok copied from the via's first element → true).
    #[tokio::test]
    async fn via_recursion_kept() {
        let ctx = ctx_from_env().await;
        let expected = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"via":[{"prop":[{"pos":"adj-i","type":"Causative"}],"reading":"悪どい 【あくどい】","gloss":[{"pos":"[adj-i]","gloss":"gaudy; showy; garish; loud"},{"pos":"[adj-i]","gloss":"crooked; vicious; wicked; nasty; unscrupulous; dishonest"}],"readok":true}],"readok":true}]"#;
        let result = conj_info_json(
            &ctx,
            10670519,
            None,
            FilterPropsText::One("あくどくさせた"),
            false,
        )
        .await
        .unwrap();
        assert_eq!(json(&result), expected);
    }

    /// REPL: `(conj-info-json 1156880 :conjugations nil :text "慰め")`. Both
    /// via-null entries resolve (readok true), so the filtered list equals
    /// the full two-entry list.
    #[tokio::test]
    async fn multi_entry_all_kept() {
        let ctx = ctx_from_env().await;
        let both = r#"[{"prop":[{"pos":"v1","type":"Continuative (~i)"}],"reading":"慰める 【なぐさめる】","gloss":[{"pos":"[vt,v1]","gloss":"to comfort; to console; to amuse"}],"readok":true},{"prop":[{"pos":"v5m","type":"Imperative"}],"reading":"慰む 【なぐさむ】","gloss":[{"pos":"[v5m,vi]","gloss":"to feel comforted; to be in good spirits; to feel better; to forget one's worries"},{"pos":"[vt,v5m]","gloss":"to trifle with; to fool around with"}],"readok":true}]"#;
        let result = conj_info_json(&ctx, 1156880, None, FilterPropsText::One("慰め"), false)
            .await
            .unwrap();
        assert_eq!(json(&result), both);
    }
}

mod simplify_reading_list {
    use crate::dict::conj::*;

    fn srl(readings: &[&str]) -> Vec<String> {
        let owned: Vec<String> = readings.iter().map(|reading| reading.to_string()).collect();
        simplify_reading_list(&owned)
    }

    #[test]
    fn simplify_reading_list_fixtures() {
        // REPL fixtures (.103, ichiran/dict::simplify-reading-list), 2026-05-23.
        let cases: &[(&[&str], &[&str])] = &[
            (&[], &[]),
            (&["aru"], &["aru"]),
            (&["tokoro ga"], &["tokoro ga"]),
            // two boundaries, single reading -> both agree -> spaces.
            (&["a b c"], &["a b c"]),
            // consecutive spaces collapse to one boundary (per-reading dedup).
            (&["a  b"], &["a b"]),
            // same de-spaced text, boundary disagrees -> MIDDLE_DOT.
            (&["tokoroga", "tokoro ga"], &["tokoro\u{00B7}ga"]),
            // same de-spaced text, boundary agrees -> space.
            (&["tokoro ga", "tokoro ga"], &["tokoro ga"]),
            // distinct de-spaced texts -> two outputs.
            (&["hito", "kuni"], &["hito", "kuni"]),
            // 2 of 3 readings split at the boundary (count<cnt) -> MIDDLE_DOT.
            (&["a b", "ab", "a b"], &["a\u{00B7}b"]),
            // leading space -> boundary at position 0.
            (&[" ab"], &[" ab"]),
            // trailing space -> boundary at length, never emitted.
            (&["ab "], &["ab"]),
            (&["a b c", "a b c", "a b c"], &["a b c"]),
            // same de-spaced "abc", two different boundaries, neither shared.
            (&["a bc", "ab c"], &["a\u{00B7}b\u{00B7}c"]),
        ];
        for (readings, expected) in cases {
            let actual = srl(readings);
            let expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
            assert_eq!(actual, expected, "readings={readings:?}");
        }
    }
}
