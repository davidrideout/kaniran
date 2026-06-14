#[cfg(feature = "loaders")]
mod recalc_entry_stats {
    use crate::dict::dao::*;

    // Affected count is the number of matched entry rows, not changed
    // rows; on a consistent dictionary the recalc rewrites each to the
    // same value. Needs a live database.
    #[test]
    fn affected_count_matches_matched_rows() {
        let ctx = KaniranContext::from_env().expect("ctx");

        // One present seq -> 1 row affected.
        let one = recalc_entry_stats(&ctx, &[1591050]).expect("one");
        assert_eq!(one, 1);

        // Three present seqs -> 3.
        let multi = recalc_entry_stats(&ctx, &[1591050, 1495740, 1221520])
            
            .expect("multi");
        assert_eq!(multi, 3);

        // Empty set -> 0.
        let empty = recalc_entry_stats(&ctx, &[]).expect("empty");
        assert_eq!(empty, 0);

        // A seq with no matching entry row -> 0 affected.
        let missing = recalc_entry_stats(&ctx, &[99999999])
            
            .expect("missing");
        assert_eq!(missing, 0);

        // Mixed present/absent -> only the present seq counts (1).
        let mixed = recalc_entry_stats(&ctx, &[1591050, 99999999])
            
            .expect("mixed");
        assert_eq!(mixed, 1);
    }

    #[test]
    fn stats_match_child_counts_after_recalc() {
        let ctx = KaniranContext::from_env().expect("ctx");

        // Varied vocabulary spanning the kanji/kana count combinations.
        // seq -> (n_kanji, n_kana)
        let cases: &[(i32, i32, i32)] = &[
            (1603990, 2, 1), // 仄か
            (1000580, 2, 2), // 彼
            (1582710, 1, 2),
            (2028930, 0, 1), // が
            (1467640, 1, 2),
        ];
        let seqs: Vec<i32> = cases.iter().map(|(seq, _, _)| *seq).collect();

        let affected = recalc_entry_stats(&ctx, &seqs).expect("recalc");
        assert_eq!(affected, seqs.len() as u64);

        for (seq, exp_kanji, exp_kana) in cases {
            let (n_kanji, n_kana): (i32, i32) =
                sqlx::query_as("SELECT n_kanji, n_kana FROM entry WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(ctx.pool.as_ref().expect("postgres pool"))
                    
                    .expect("entry row");
            assert_eq!(n_kanji, *exp_kanji, "seq={seq} n_kanji");
            assert_eq!(n_kana, *exp_kana, "seq={seq} n_kana");

            let actual_kanji: i64 =
                sqlx::query_scalar("SELECT COUNT(id) FROM kanji_text WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(ctx.pool.as_ref().expect("postgres pool"))
                    
                    .expect("kanji count");
            let actual_kana: i64 =
                sqlx::query_scalar("SELECT COUNT(id) FROM kana_text WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(ctx.pool.as_ref().expect("postgres pool"))
                    
                    .expect("kana count");
            assert_eq!(
                n_kanji as i64, actual_kanji,
                "seq={seq} stored vs actual kanji"
            );
            assert_eq!(
                n_kana as i64, actual_kana,
                "seq={seq} stored vs actual kana"
            );
        }
    }
}

#[cfg(feature = "loaders")]
mod recalc_entry_stats_all {
    use crate::dict::dao::*;

    // Affected count is the total entry row count (matched rows, not
    // changed rows), and afterwards every entry's stored stats equal its
    // child-row counts. Needs a live database.
    #[test]
    fn affects_all_entries_and_stats_match_children() {
        let ctx = KaniranContext::from_env().expect("ctx");

        let affected = recalc_entry_stats_all(&ctx).expect("recalc-all");

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entry")
            .fetch_one(ctx.pool.as_ref().expect("postgres pool"))
            
            .expect("count entries");
        assert_eq!(affected, total as u64, "affected != total entry rows");

        // Spot-check varied vocabulary post-recalc: stored stats equal
        // the independently-counted child rows.
        // seq -> (n_kanji, n_kana)
        let cases: &[(i32, i32, i32)] = &[
            (1603990, 2, 1), // 仄か
            (1000580, 2, 2), // 彼
            (1582710, 1, 2),
            (1591050, 2, 1), // 気が付く
            (2028930, 0, 1), // が
            (1467640, 1, 2),
        ];
        for (seq, exp_kanji, exp_kana) in cases {
            let (n_kanji, n_kana): (i32, i32) =
                sqlx::query_as("SELECT n_kanji, n_kana FROM entry WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(ctx.pool.as_ref().expect("postgres pool"))
                    
                    .expect("entry row");
            assert_eq!(n_kanji, *exp_kanji, "seq={seq} n_kanji");
            assert_eq!(n_kana, *exp_kana, "seq={seq} n_kana");

            let actual_kanji: i64 =
                sqlx::query_scalar("SELECT COUNT(id) FROM kanji_text WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(ctx.pool.as_ref().expect("postgres pool"))
                    
                    .expect("kanji count");
            let actual_kana: i64 =
                sqlx::query_scalar("SELECT COUNT(id) FROM kana_text WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(ctx.pool.as_ref().expect("postgres pool"))
                    
                    .expect("kana count");
            assert_eq!(
                n_kanji as i64, actual_kanji,
                "seq={seq} stored vs actual kanji"
            );
            assert_eq!(
                n_kana as i64, actual_kana,
                "seq={seq} stored vs actual kana"
            );
        }
    }
}

mod entry_digest {
    use crate::dict::dao::*;

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[cfg(feature = "postgres")]
    fn load_entry(ctx: &KaniranContext, seq: i32) -> Entry {
        tokio::runtime::Runtime::new()
            .expect("tokio runtime")
            .block_on(
                sqlx::query_as::<_, Entry>("SELECT * FROM entry WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(ctx.pool.as_ref().expect("postgres pool")),
            )
            .unwrap()
    }

    /// Entry digest for kanji-bearing entries (text comes from the kanji
    /// form) and kana-only entries (text equals the kana).
    #[cfg(feature = "postgres")]
    #[test]
    fn entry_digest_fixtures() {
        let ctx = ctx_from_env();
        let cases: &[(i32, &str, &str)] = &[
            (1257590, "憲法", "けんぽう"),
            (1386690, "雪崩", "なだれ"),
            (1573390, "躊躇う", "ためらう"),
            (1087690, "ドーナツ", "ドーナツ"),
            (1010900, "ぴったり", "ぴったり"),
        ];
        for (seq, text, kana) in cases {
            let entry = load_entry(&ctx, *seq);
            let digest = entry_digest(&ctx, &entry).unwrap();
            assert_eq!(
                (digest.0, digest.1.as_deref(), digest.2.as_deref()),
                (*seq, Some(*text), Some(*kana)),
                "seq={seq}"
            );
        }
    }
}

mod conj_info_short {
    use crate::dict::dao::*;

    fn prop(pos: &str, conj_type: i32, neg: Option<bool>, fml: Option<bool>) -> ConjProp {
        ConjProp {
            id: 0,
            conj_id: 0,
            conj_type,
            pos: pos.to_string(),
            neg,
            fml,
        }
    }

    /// Short conjugation description across every negative/formal state
    /// (false, true, and database-null), plus a missing description that
    /// renders as "NIL".
    #[test]
    fn conj_info_short_fixtures() {
        let cases: &[(ConjProp, &str)] = &[
            (
                prop("v5k", 2, Some(false), Some(false)),
                "[v5k] Past (~ta) Affirmative Plain",
            ),
            (
                prop("v5k", 1, Some(false), Some(true)),
                "[v5k] Non-past Affirmative Formal",
            ),
            (
                prop("v5k", 1, Some(true), Some(false)),
                "[v5k] Non-past Negative Plain",
            ),
            (
                prop("v5k", 1, Some(true), Some(true)),
                "[v5k] Non-past Negative Formal",
            ),
            (
                prop("adj-i", 3, Some(false), None),
                "[adj-i] Conjunctive (~te) Affirmative",
            ),
            (
                prop("v5k", 52, Some(true), None),
                "[v5k] Negative Stem Negative",
            ),
            (
                prop("adj-i", 9, None, Some(false)),
                "[adj-i] Volitional Plain",
            ),
            (
                prop("adj-i", 9, None, Some(true)),
                "[adj-i] Volitional Formal",
            ),
            (prop("v5k", 13, None, None), "[v5k] Continuative (~i)"),
            (
                prop("v5k", 999, Some(false), Some(false)),
                "[v5k] NIL Affirmative Plain",
            ),
        ];
        for (obj, expected) in cases {
            assert_eq!(&conj_info_short(obj), expected, "obj={obj:?}");
        }
    }
}

mod conj_prop_json {
    use crate::dict::dao::*;

    fn prop(conj_type: i32, pos: &str, neg: Option<bool>, fml: Option<bool>) -> ConjProp {
        ConjProp {
            id: 0,
            conj_id: 0,
            conj_type,
            pos: pos.to_owned(),
            neg,
            fml,
        }
    }

    /// Conjugation-property JSON across every negative/formal state
    /// (null, true, false); the no-description row renders "type" as an
    /// empty list.
    #[test]
    fn conj_prop_json_fixtures() {
        let cases = [
            // neg null, fml null
            (
                prop(13, "v5k", None, None),
                r#"{"pos":"v5k","type":"Continuative (~i)"}"#,
            ),
            // neg true, fml true
            (
                prop(1, "v5k", Some(true), Some(true)),
                r#"{"pos":"v5k","type":"Non-past","neg":true,"fml":true}"#,
            ),
            // neg true, fml false
            (
                prop(1, "v5k", Some(true), Some(false)),
                r#"{"pos":"v5k","type":"Non-past","neg":true}"#,
            ),
            // neg false, fml true
            (
                prop(1, "v5k", Some(false), Some(true)),
                r#"{"pos":"v5k","type":"Non-past","fml":true}"#,
            ),
            // neg false, fml false
            (
                prop(2, "v5k", Some(false), Some(false)),
                r#"{"pos":"v5k","type":"Past (~ta)"}"#,
            ),
            // neg null, fml true
            (
                prop(9, "adj-i", None, Some(true)),
                r#"{"pos":"adj-i","type":"Volitional","fml":true}"#,
            ),
            // neg true, fml null
            (
                prop(52, "v5k", Some(true), None),
                r#"{"pos":"v5k","type":"Negative Stem","neg":true}"#,
            ),
            // no description for conj_type → "type" renders as []
            (prop(999, "v5k", None, None), r#"{"pos":"v5k","type":[]}"#),
        ];
        for (obj, expected) in &cases {
            let actual = serde_json::to_string(&conj_prop_json(obj)).unwrap();
            assert_eq!(
                actual.as_str(),
                *expected,
                "conj_type={} pos={}",
                obj.conj_type,
                obj.pos
            );
        }
    }
}
