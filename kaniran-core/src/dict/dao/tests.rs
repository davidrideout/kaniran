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
