mod select_conjs {
    use crate::dict::conj::*;
    use std::sync::Arc;

    fn ctx_from_env() -> Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// With no conjugation filter, prefers the rows whose "via" is empty, and
    /// only when there are none does it fall back to returning all rows.
    /// - 2028980: one via-empty row.
    /// - 1156880: two via-empty rows (a via-set row exists but is excluded).
    /// - 1257260: no via-empty rows, so all rows come back (both via-set).
    #[test]
    fn select_conjs_nil_conj_ids() {
        let ctx = ctx_from_env();

        // Conj-table ids renumber per build, so pin each result by its stable
        // shape: row count, the base seq(s) it derives from, and via-presence.
        let seq_froms = |rows: &[Conjugation]| {
            let mut out: Vec<i32> = rows.iter().map(|conj| conj.seq_from).collect();
            out.sort_unstable();
            out
        };

        let r2028980 = select_conjs(&ctx, 2028980, None).unwrap();
        assert_eq!(r2028980.len(), 1);
        assert_eq!(r2028980[0].seq_from, 2089020);
        assert_eq!(r2028980[0].seq_via, None);

        let r1156880 = select_conjs(&ctx, 1156880, None).unwrap();
        assert_eq!(seq_froms(&r1156880), vec![1156870, 1156890]);
        assert!(r1156880.iter().all(|c| c.seq_via.is_none()));

        // No via-empty rows, so all rows come back (both via-set).
        let r1257260 = select_conjs(&ctx, 1257260, None).unwrap();
        assert_eq!(r1257260.len(), 2);
        assert!(r1257260.iter().all(|c| c.seq_via.is_some()));
    }

    #[test]
    fn select_conjs_root_is_empty() {
        let ctx = ctx_from_env();
        let result = select_conjs(&ctx, 2028980, Some(&WordConjugations::Root))
            
            .unwrap();
        assert!(result.is_empty());
    }

    /// An explicit id list returns exactly those ids, with no via-empty
    /// preference and no fallback — including a via-set row that the
    /// unfiltered path would otherwise exclude.
    #[test]
    fn select_conjs_explicit_ids() {
        let ctx = ctx_from_env();

        // Conj ids renumber per build, so resolve them from stable anchors.
        let v5m_id = crate::test_support::conj_id_by_pos(1156880, "v5m");
        let one = select_conjs(&ctx, 1156880, Some(&WordConjugations::Ids(vec![v5m_id])))

            .unwrap();
        let ids: Vec<i32> = one.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![v5m_id]);
        assert_eq!(one[0].seq_via, None);

        // A via-set row is selectable by id even though the unfiltered path
        // would exclude it.
        let via_id = crate::test_support::via_set_conj_id(1156880);
        let via_row = select_conjs(&ctx, 1156880, Some(&WordConjugations::Ids(vec![via_id])))

            .unwrap();
        assert_eq!(via_row.len(), 1);
        assert_eq!(via_row[0].seq_via, Some(1156890));

        // Ids that don't belong to the seq are filtered out.
        let none = select_conjs(&ctx, 1156880, Some(&WordConjugations::Ids(vec![1])))
            
            .unwrap();
        assert!(none.is_empty());
    }
}

mod conj_type_order {
    use crate::dict::conj::*;

    /// Conjugation types 10 and 13 swap with each other; every other value
    /// maps to itself.
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

    /// True when the text ends in one of the four rareru suffixes. A rareru
    /// substring that isn't at the end (`られるよ`) is false; the empty string
    /// is false.
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

    /// Drops the passive props (ids 1, 4, 5) only when the text is present
    /// and is not a rareru form. With no text, a rareru form, or an empty
    /// list (treated as no text), everything is kept; the empty string still
    /// counts as present and triggers the drop.
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
        // Empty props in → empty result.
        assert!(filter_props(&[], FilterPropsText::One("食べる")).is_empty());
    }
}

mod select_conjs_and_props {
    use crate::dict::conj::*;
    use std::sync::Arc;

    fn ctx_from_env() -> Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    // Conj/prop ids and the synthetic `seq_via` target renumber per build, so
    // the projection drops them and keeps the stable shape: the base seq, the
    // source seq, whether a via is present, the sort key, and each prop's
    // (conj_type, pos, neg, fml).
    type FpropRow = (i32, String, Option<bool>, Option<bool>);
    type ConjRow = (i32, i32, bool, [i32; 2], Vec<FpropRow>);

    fn project(rows: &[(Conjugation, Vec<ConjProp>, [i32; 2])]) -> Vec<ConjRow> {
        rows.iter()
            .map(|(conj, fprops, key)| {
                (
                    conj.seq,
                    conj.seq_from,
                    conj.seq_via.is_some(),
                    *key,
                    fprops
                        .iter()
                        .map(|p| (p.conj_type, p.pos.clone(), p.neg, p.fml))
                        .collect(),
                )
            })
            .collect()
    }

    /// Two via-empty conjugations come back sorted by their conjugation-type
    /// order, which reorders the input: conj 661748 (type 13, ordered as 10)
    /// sorts ahead of conj 366552 (type 10, ordered as 13). No text, so all
    /// props are kept.
    #[test]
    fn via_null_sorted_by_val() {
        let ctx = ctx_from_env();
        let rows = select_conjs_and_props(&ctx, 1156880, None, FilterPropsText::None)
            
            .unwrap();
        let expected: Vec<ConjRow> = vec![
            (
                1156880,
                1156890,
                false,
                [0, 10],
                vec![(13, "v1".to_string(), None, None)],
            ),
            (
                1156880,
                1156870,
                false,
                [0, 13],
                vec![(10, "v5m".to_string(), Some(false), Some(false))],
            ),
        ];
        assert_eq!(project(&rows), expected);
    }

    /// When there are no via-empty rows the fallback returns all rows; both
    /// have a via set, so each sort key leads with 1, and they sort by
    /// conjugation-type order within that.
    #[test]
    fn via_not_null_flag_one() {
        let ctx = ctx_from_env();
        let rows = select_conjs_and_props(&ctx, 1257260, None, FilterPropsText::None)
            
            .unwrap();
        let expected: Vec<ConjRow> = vec![
            (
                1257260,
                1609260,
                true,
                [1, 10],
                vec![(13, "v1".to_string(), None, None)],
            ),
            (
                1257260,
                1609260,
                true,
                [1, 13],
                vec![(10, "v5s".to_string(), Some(false), Some(false))],
            ),
        ];
        assert_eq!(project(&rows), expected);
    }

    /// The text argument is forwarded to the prop filter: the passive prop is
    /// dropped exactly when the filter would drop it (text present, not a
    /// rareru form). The sort key is computed from the unfiltered props, so it
    /// stays the same across every text variant.
    ///
    /// Postgres-only: the conjugation it exercises (seq 1232500, deriving from
    /// 2864818) is not present in the rkyv archive's JMdict vintage, so
    /// `select_conjs_and_props` returns nothing there. The prop-filtering logic
    /// itself is covered backend-agnostically by the `filter_props` tests.
    #[cfg(feature = "postgres")]
    #[test]
    fn text_threads_to_filter_props() {
        let ctx = ctx_from_env();
        let prop = (6, "v1".to_string(), Some(false), Some(false));
        let kept: Vec<ConjRow> = vec![(1232500, 2864818, false, [0, 6], vec![prop.clone()])];
        let dropped: Vec<ConjRow> = vec![(1232500, 2864818, false, [0, 6], vec![])];

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
                
                .unwrap();
            assert_eq!(&project(&rows), *expected, "text variant mismatch");
        }
    }

    /// The root conjugation filter yields no conjugations, so the result is
    /// empty.
    #[test]
    fn root_conj_ids_empty() {
        let ctx = ctx_from_env();
        let rows = select_conjs_and_props(
            &ctx,
            2028980,
            Some(&WordConjugations::Root),
            FilterPropsText::None,
        )
        
        .unwrap();
        assert!(rows.is_empty());
    }
}

mod print_conj_info {
    use crate::dict::conj::*;
    use std::sync::Arc;

    fn ctx_from_env() -> Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    fn render(
        ctx: &KaniranContext,
        seq: i32,
        conjugations: Option<&WordConjugations>,
    ) -> String {
        let mut out = String::new();
        print_conj_info(ctx, seq, conjugations, &mut out)
            
            .unwrap();
        out
    }

    /// Renders the conjugation breakdown for a sequence. Covers:
    /// - 1156880: two via-empty conjugations, one prop each.
    /// - 1184270: one via-empty conjugation with two props in a single block.
    /// - 1257260: two via-set conjugations, each printing a "--(via)--" entry.
    /// - 10674648: two conjugations sharing the same via; the second via is
    ///   suppressed, so its block prints only once.
    /// - 1358280: no conjugations, so output is empty.
    #[test]
    fn print_conj_info_fixtures() {
        let ctx = ctx_from_env();
        // 10674648 is a synthetic doubly-conjugated entry (renumbers per
        // build); its surface くねらせた is stable, so resolve the seq from it.
        let kuneraseta = crate::test_support::conj_entry_seq("くねらせた");
        let cases: Vec<(i32, &str)> = vec![
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
                kuneraseta,
                "\n[ Conjugation: [v1] Past (~ta) Affirmative Plain\n --(via)--\n[ Conjugation: [v5s] Potential Affirmative Plain\n  くねらす : to wriggle; to twist (one's body); to writhe ]\n[ Conjugation: [v5r] Causative Affirmative Plain\n  くねる : to bend loosely back and forth; to wriggle; to be crooked ] ]",
            ),
            (1358280, ""),
        ];
        for (seq, expected) in &cases {
            assert_eq!(&render(&ctx, *seq, None), expected, "seq={seq}");
        }
    }

    /// The conjugation filter narrows the output: root selects nothing (empty
    /// output), an explicit id list prints just that one conjugation.
    #[test]
    fn print_conj_info_conjugations_arg() {
        let ctx = ctx_from_env();
        assert_eq!(
            render(&ctx, 1156880, Some(&WordConjugations::Root)),
            "",
            "conjugations=:root"
        );
        let v5m_id = crate::test_support::conj_id_by_pos(1156880, "v5m");
        assert_eq!(
            render(&ctx, 1156880, Some(&WordConjugations::Ids(vec![v5m_id]))),
            "\n[ Conjugation: [v5m] Imperative Affirmative Plain\n  慰む 【なぐさむ】 : to feel comforted; to be in good spirits; to feel better; to forget one's worries ]",
            "conjugations=v5m"
        );
    }
}

mod conj_info_json_star_ {
    use crate::dict::conj::*;
    use std::sync::Arc;

    fn ctx_from_env() -> Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    fn json(values: &[Value]) -> String {
        serde_json::to_string(values).unwrap()
    }

    /// For a via-empty conjugation, a kana or kanji surface that resolves the
    /// original reading sets readok true. A non-matching surface or no text
    /// leaves readok empty. When the reading can't be resolved and a gloss is
    /// required, the entry is dropped entirely.
    #[test]
    fn via_null_paths() {
        let ctx = ctx_from_env();
        // The conjugated-entry seq renumbers per build; its surface 尽き果てた
        // is stable, so resolve the input from it.
        let seq = crate::test_support::conj_entry_seq("尽き果てた");
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
            let result = conj_info_json_star_(&ctx, seq, None, case.text, case.has_gloss)

                .unwrap();
            assert_eq!(json(&result), case.expected, "case={}", case.label);
        }
    }

    /// A via-set conjugation nests the via's own conjugation info under
    /// `"via"` and copies its readok up to the outer entry.
    #[test]
    fn via_not_null_recursion() {
        let ctx = ctx_from_env();
        let seq = crate::test_support::conj_entry_seq("あくどくさせた");
        let expected = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"via":[{"prop":[{"pos":"adj-i","type":"Causative"}],"reading":"悪どい 【あくどい】","gloss":[{"pos":"[adj-i]","gloss":"gaudy; showy; garish; loud"},{"pos":"[adj-i]","gloss":"crooked; vicious; wicked; nasty; unscrupulous; dishonest"}],"readok":true}],"readok":true}]"#;
        for has_gloss in [false, true] {
            let result = conj_info_json_star_(
                &ctx,
                seq,
                None,
                FilterPropsText::One("あくどくさせた"),
                has_gloss,
            )
            
            .unwrap();
            assert_eq!(json(&result), expected, "has_gloss={has_gloss}");
        }
    }

    /// A sequence with two via-empty conjugations emits two entries. No text
    /// leaves both readings unresolved (`readok` empty); restricting to one
    /// conjugation id emits a single entry.
    #[test]
    fn multi_entry_and_conj_ids() {
        let ctx = ctx_from_env();
        let both = r#"[{"prop":[{"pos":"v1","type":"Continuative (~i)"}],"reading":"慰める 【なぐさめる】","gloss":[{"pos":"[vt,v1]","gloss":"to comfort; to console; to amuse"}],"readok":true},{"prop":[{"pos":"v5m","type":"Imperative"}],"reading":"慰む 【なぐさむ】","gloss":[{"pos":"[v5m,vi]","gloss":"to feel comforted; to be in good spirits; to feel better; to forget one's worries"},{"pos":"[vt,v5m]","gloss":"to trifle with; to fool around with"}],"readok":true}]"#;
        let both_unresolved = r#"[{"prop":[{"pos":"v1","type":"Continuative (~i)"}],"reading":"慰める 【なぐさめる】","gloss":[{"pos":"[vt,v1]","gloss":"to comfort; to console; to amuse"}],"readok":[]},{"prop":[{"pos":"v5m","type":"Imperative"}],"reading":"慰む 【なぐさむ】","gloss":[{"pos":"[v5m,vi]","gloss":"to feel comforted; to be in good spirits; to feel better; to forget one's worries"},{"pos":"[vt,v5m]","gloss":"to trifle with; to fool around with"}],"readok":[]}]"#;
        let only_one = r#"[{"prop":[{"pos":"v1","type":"Continuative (~i)"}],"reading":"慰める 【なぐさめる】","gloss":[{"pos":"[vt,v1]","gloss":"to comfort; to console; to amuse"}],"readok":true}]"#;

        let result = conj_info_json_star_(&ctx, 1156880, None, FilterPropsText::One("慰め"), false)
            
            .unwrap();
        assert_eq!(json(&result), both, "慰め");

        let result = conj_info_json_star_(&ctx, 1156880, None, FilterPropsText::None, false)
            
            .unwrap();
        assert_eq!(json(&result), both_unresolved, "nil text");

        // Restrict to the v1 Continuative conjugation; its conj id renumbers
        // per build, so resolve it from the stable pos.
        let ids = WordConjugations::Ids(vec![crate::test_support::conj_id_by_pos(1156880, "v1")]);
        let result = conj_info_json_star_(
            &ctx,
            1156880,
            Some(&ids),
            FilterPropsText::One("慰め"),
            false,
        )

        .unwrap();
        assert_eq!(json(&result), only_one, "conj-ids v1");
    }
}

mod conj_info_json {
    use crate::dict::conj::*;
    use std::sync::Arc;

    fn ctx_from_env() -> Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    fn json(values: &[Value]) -> String {
        serde_json::to_string(values).unwrap()
    }

    /// A matching surface resolves the original reading (readok true, entry
    /// kept). With no text every entry's readok is empty, so the filtered list
    /// comes back empty and the result falls back to the unfiltered list.
    #[test]
    fn readok_filter_and_fallback() {
        let ctx = ctx_from_env();
        // The conjugated-entry seq renumbers per build; resolve it from the
        // stable surface 尽き果てた.
        let seq = crate::test_support::conj_entry_seq("尽き果てた");
        let found = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"reading":"尽き果てる 【つきはてる】","gloss":[{"pos":"[vi,v1]","gloss":"to be exhausted"}],"readok":true}]"#;
        let unresolved = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"reading":"尽き果てる 【つきはてる】","gloss":[{"pos":"[vi,v1]","gloss":"to be exhausted"}],"readok":[]}]"#;

        let result = conj_info_json(
            &ctx,
            seq,
            None,
            FilterPropsText::One("つきはてた"),
            false,
        )

        .unwrap();
        assert_eq!(json(&result), found, "resolved surface");

        let result = conj_info_json(&ctx, seq, None, FilterPropsText::None, false)

            .unwrap();
        assert_eq!(json(&result), unresolved, "nil text → fallback to cij");

        // has-gloss plus an unresolved reading drops the only entry, so the
        // result is the empty list.
        let result = conj_info_json(
            &ctx,
            seq,
            None,
            FilterPropsText::One("存在しない"),
            true,
        )
        
        .unwrap();
        assert_eq!(json(&result), "[]", "has-gloss drop → empty");
    }

    /// A via-set entry keeps its nested `via` payload, with readok copied up
    /// from the via's first element.
    #[test]
    fn via_recursion_kept() {
        let ctx = ctx_from_env();
        let seq = crate::test_support::conj_entry_seq("あくどくさせた");
        let expected = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"via":[{"prop":[{"pos":"adj-i","type":"Causative"}],"reading":"悪どい 【あくどい】","gloss":[{"pos":"[adj-i]","gloss":"gaudy; showy; garish; loud"},{"pos":"[adj-i]","gloss":"crooked; vicious; wicked; nasty; unscrupulous; dishonest"}],"readok":true}],"readok":true}]"#;
        let result = conj_info_json(
            &ctx,
            seq,
            None,
            FilterPropsText::One("あくどくさせた"),
            false,
        )
        
        .unwrap();
        assert_eq!(json(&result), expected);
    }

    /// When both via-empty entries resolve, the filtered list equals the full
    /// two-entry list.
    #[test]
    fn multi_entry_all_kept() {
        let ctx = ctx_from_env();
        let both = r#"[{"prop":[{"pos":"v1","type":"Continuative (~i)"}],"reading":"慰める 【なぐさめる】","gloss":[{"pos":"[vt,v1]","gloss":"to comfort; to console; to amuse"}],"readok":true},{"prop":[{"pos":"v5m","type":"Imperative"}],"reading":"慰む 【なぐさむ】","gloss":[{"pos":"[v5m,vi]","gloss":"to feel comforted; to be in good spirits; to feel better; to forget one's worries"},{"pos":"[vt,v5m]","gloss":"to trifle with; to fool around with"}],"readok":true}]"#;
        let result = conj_info_json(&ctx, 1156880, None, FilterPropsText::One("慰め"), false)
            
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
