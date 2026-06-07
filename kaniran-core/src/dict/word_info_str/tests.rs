mod reading_str_star_ {
    use crate::dict::word_info_str::*;

    /// REPL fixtures (.103, `ichiran/dict::reading-str*`), 2026-05-24.
    /// Covers kanji+kana, kana-only (kanji nil → bare kana), kanji with
    /// nil kana (`~a` of nil → "NIL"), and both nil → nil.
    #[test]
    fn reading_str_star_fixtures() {
        let cases: &[(Option<&str>, Option<&str>, Option<&str>)] = &[
            (Some("日本"), Some("にほん"), Some("日本 【にほん】")),
            (None, Some("ねこ"), Some("ねこ")),
            (Some("猫"), None, Some("猫 【NIL】")),
            (None, None, None),
        ];
        for (kanji, kana, expected) in cases {
            assert_eq!(
                reading_str_star_(*kanji, *kana).as_deref(),
                *expected,
                "kanji={kanji:?} kana={kana:?}"
            );
        }
    }
}

mod reading_str_seq {
    use crate::dict::word_info_str::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::reading-str-seq`), 2026-05-24.
    /// Covers a kanji+kana entry, a conjugating kanji+kana verb, two
    /// kana-only entries (no kanji-text row → kanji nil → bare kana),
    /// and an unknown seq (both column queries empty → nil).
    #[tokio::test]
    async fn reading_str_seq_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, Option<&str>)] = &[
            (1582710, Some("日本 【にほん】")),
            (1358280, Some("食べる 【たべる】")),
            (1010890, Some("ぴちぴち")),
            (1056070, Some("サイバネーション")),
            (999999, None),
        ];
        for (seq, expected) in cases {
            assert_eq!(
                reading_str_seq(&ctx, *seq).await.unwrap().as_deref(),
                *expected,
                "seq={seq}"
            );
        }
    }
}

mod entry_info_short {
    use crate::dict::word_info_str::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::entry-info-short`), 2026-05-24.
    /// Covers a noun (1582710 日本) and a verb (1358280 食べる) with no
    /// with-pos, with-pos matching the entry's pos, and with-pos that
    /// matches no sense (→ sense-str nil → trailing nothing after " : "),
    /// plus an unknown seq (reading nil → "NIL").
    #[tokio::test]
    async fn entry_info_short_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, Option<&str>, &str)] = &[
            (1582710, None, "日本 【にほん】 : Japan"),
            (1358280, None, "食べる 【たべる】 : to eat"),
            (1358280, Some("v1"), "食べる 【たべる】 : to eat"),
            (1358280, Some("n"), "食べる 【たべる】 : "),
            (1582710, Some("v1"), "日本 【にほん】 : "),
            (999999, None, "NIL : "),
            (999999, Some("v1"), "NIL : "),
        ];
        for (seq, with_pos, expected) in cases {
            assert_eq!(
                entry_info_short(&ctx, *seq, *with_pos).await.unwrap(),
                *expected,
                "seq={seq} with_pos={with_pos:?}"
            );
        }
    }
}

mod entry_info_long {
    use crate::dict::word_info_str::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::entry-info-long`), 2026-05-25.
    /// Covers a single-sense noun (reading with 【kanji】), a verb,
    /// a kana-only entry (reading-str* returns bare kana, no 【】),
    /// a multi-sense entry exercising the `《s_inf》` annotation and the
    /// `get-senses-raw` last-tag-group reversal (sense 4 pos), and an
    /// unknown seq (reading nil + empty senses → just the seq, the
    /// `~@[` nil branch).
    #[tokio::test]
    async fn entry_info_long_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, &str)] = &[
            (1562520, "1562520 賄賂 【わいろ】\n1. [n] bribe; sweetener; douceur"),
            (1573390, "1573390 躊躇う 【ためらう】\n1. [vi,v5u] to hesitate; to waver"),
            (1087690, "1087690 ドーナツ\n1. [n] doughnut; donut"),
            (
                1010900,
                "1010900 ぴったり\n1. [adv,adv-to,vs] 《ぴったし is colloquial》 tightly; closely\n2. [adv,adv-to,vs] exactly; precisely\n3. [adv,adv-to,vs] suddenly (stopping)\n4. [adj-na,vs,adv-to,adv] perfectly (suited); ideally",
            ),
            (999999, "999999"),
        ];
        for (seq, expected) in cases {
            assert_eq!(
                &entry_info_long(&ctx, *seq).await.unwrap(),
                expected,
                "seq={seq}"
            );
        }
    }
}

mod map_word_info_kana {
    use crate::dict::word_info_str::*;

    fn wi(kana: Option<WordInfoKana>) -> WordInfo {
        WordInfo {
            kana,
            ..Default::default()
        }
    }

    fn single(text: &str) -> Option<WordInfoKana> {
        Some(WordInfoKana::Single(text.to_string()))
    }

    /// `fn` analogous to the REPL `string-upcase` probe.
    fn upcase(element: &Option<WordInfoKana>) -> String {
        match element {
            Some(WordInfoKana::Single(text)) => text.to_uppercase(),
            other => format!("{other:?}"),
        }
    }

    /// `fn` analogous to the REPL `identity` probe (kana element as text).
    fn ident(element: &Option<WordInfoKana>) -> String {
        match element {
            Some(WordInfoKana::Single(text)) => text.clone(),
            other => format!("{other:?}"),
        }
    }

    #[test]
    fn map_word_info_kana_fixtures() {
        // REPL fixtures (.103, ichiran/dict::map-word-info-kana), 2026-05-23.

        // String branch: (funcall fn wkana).
        assert_eq!(map_word_info_kana(upcase, &wi(single("neko")), "/"), "NEKO");

        // List branch, default separator "/".
        let inu = Some(WordInfoKana::Multi(vec![single("neko"), single("inu")]));
        assert_eq!(
            map_word_info_kana(upcase, &wi(inu.clone()), "/"),
            "NEKO/INU"
        );

        // List branch, identity fn -> simplify-reading-list merges the
        // shared de-spaced reading with a MIDDLE_DOT boundary.
        let merge = Some(WordInfoKana::Multi(vec![single("a b"), single("ab")]));
        assert_eq!(map_word_info_kana(ident, &wi(merge), "/"), "a\u{00B7}b");

        // nil kana -> list branch -> "".
        assert_eq!(map_word_info_kana(upcase, &wi(None), "/"), "");

        // Separator override.
        assert_eq!(map_word_info_kana(ident, &wi(inu), "+"), "neko+inu");

        // Single-element list.
        let one = Some(WordInfoKana::Multi(vec![single("neko")]));
        assert_eq!(map_word_info_kana(upcase, &wi(one), "/"), "NEKO");
    }
}

mod word_info_reading_str {
    use crate::dict::word_info::WordInfoSeq;
    use crate::dict::word_info_str::*;

    fn wi(
        kind: WordInfoType,
        text: &str,
        kana: Option<WordInfoKana>,
        counter: Option<(String, bool)>,
        seq: Option<WordInfoSeq>,
    ) -> WordInfo {
        WordInfo {
            kind,
            text: text.to_string(),
            kana,
            counter,
            seq,
            ..Default::default()
        }
    }

    /// REPL fixtures (.103, `ichiran/dict::word-info-reading-str`), 2026-05-24.
    /// Covers the kanji-type branch (single-string / list / nested-list-with-nil
    /// / nil kana → `~a` princ rendering), the counter+seq branch reaching the
    /// same body, and the else branch (counter without seq, kana type, gap type).
    #[test]
    fn word_info_reading_str_fixtures() {
        use WordInfoKana::{Multi, Single};
        let single = |reading: &str| Single(reading.to_string());
        let cases: Vec<(WordInfo, &str)> = vec![
            (
                wi(
                    WordInfoType::Kanji,
                    "日本",
                    Some(single("にほん")),
                    None,
                    None,
                ),
                "日本 【にほん】",
            ),
            (
                wi(
                    WordInfoType::Kanji,
                    "日本",
                    Some(Multi(vec![
                        Some(single("にほん")),
                        Some(single("にっぽん")),
                    ])),
                    None,
                    None,
                ),
                "日本 【(にほん にっぽん)】",
            ),
            (
                wi(
                    WordInfoType::Kanji,
                    "X",
                    Some(Multi(vec![
                        Some(single("あ")),
                        None,
                        Some(Multi(vec![Some(single("い")), Some(single("う"))])),
                    ])),
                    None,
                    None,
                ),
                "X 【(あ NIL (い う))】",
            ),
            (
                wi(WordInfoType::Kanji, "日本", None, None, None),
                "日本 【NIL】",
            ),
            (
                wi(
                    WordInfoType::Kana,
                    "三冊",
                    Some(single("さんさつ")),
                    Some(("3".to_string(), false)),
                    Some(WordInfoSeq::Single(12345)),
                ),
                "三冊 【さんさつ】",
            ),
            (
                wi(
                    WordInfoType::Kana,
                    "三冊",
                    Some(single("さんさつ")),
                    Some(("3".to_string(), false)),
                    None,
                ),
                "三冊",
            ),
            (
                wi(WordInfoType::Kana, "ねこ", Some(single("ねこ")), None, None),
                "ねこ",
            ),
            (
                wi(WordInfoType::Gap, "?", Some(single("?")), None, None),
                "?",
            ),
        ];
        for (word_info, expected) in &cases {
            assert_eq!(
                word_info_reading_str(word_info).as_deref(),
                Some(*expected),
                "text={:?}",
                word_info.text
            );
        }
    }
}

mod word_info_str {
    use crate::dict::word_info::{WordInfoKana, WordInfoType};
    use crate::dict::word_info_str::*;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn single(reading: &str) -> Option<WordInfoKana> {
        Some(WordInfoKana::Single(reading.to_string()))
    }

    /// REPL fixtures (.103, `(word-info-str (make-instance 'word-info …))`),
    /// 2026-05-24, after `(init-suffixes t)`. Each row builds one word-info and
    /// pins the exact output (blank lines included). Covers:
    /// - A: default branch, no conjugations → senses.
    /// - B: default branch, conjugations nil → empty senses + full conj-info.
    /// - C: conjugations `:root` → conj display suppressed (test2 still fires).
    /// - D: seq nil → "???".
    /// - E: counter + seq → value then senses.
    /// - F: counter, no seq → value only.
    /// - G: compound, non-primary suffix component → marker, suffix description.
    /// - G2: compound, non-primary component without a suffix description →
    ///   marker, falls through to senses.
    /// - H: alternative → "<i>. " prefixes, second reading a counter.
    #[tokio::test]
    async fn word_info_str_fixtures() {
        use WordInfoType::{Kana, Kanji};
        let ctx = ctx_from_env().await;

        let compound = |text: &str, kana: &str, seqs: &[i32], comps: Vec<WordInfo>| WordInfo {
            kind: Kanji,
            text: text.to_string(),
            kana: single(kana),
            seq: Some(WordInfoSeq::Multi(
                seqs.iter().map(|s| Some(WordInfoSeq::Single(*s))).collect(),
            )),
            components: comps,
            ..Default::default()
        };

        let cases: Vec<(&str, WordInfo, &str)> = vec![
            (
                "A",
                WordInfo {
                    kind: Kanji,
                    text: "日本".to_string(),
                    kana: single("にほん"),
                    seq: Some(WordInfoSeq::Single(1582710)),
                    ..Default::default()
                },
                "日本 【にほん】\n1. [n] Japan",
            ),
            (
                "B",
                WordInfo {
                    kind: Kanji,
                    text: "食べた".to_string(),
                    kana: single("たべた"),
                    seq: Some(WordInfoSeq::Single(10092229)),
                    ..Default::default()
                },
                "食べた 【たべた】\n\n[ Conjugation: [v1] Past (~ta) Affirmative Plain\n  食べる 【たべる】 : to eat ]",
            ),
            (
                "C",
                WordInfo {
                    kind: Kanji,
                    text: "食べた".to_string(),
                    kana: single("たべた"),
                    seq: Some(WordInfoSeq::Single(10092229)),
                    conjugations: Some(WordConjugations::Root),
                    ..Default::default()
                },
                "食べた 【たべた】\n",
            ),
            (
                "D",
                WordInfo {
                    kind: Kana,
                    text: "ねこねこ".to_string(),
                    kana: single("ねこねこ"),
                    seq: None,
                    ..Default::default()
                },
                "ねこねこ\n???",
            ),
            (
                "E",
                WordInfo {
                    kind: Kanji,
                    text: "三冊".to_string(),
                    kana: single("さんさつ"),
                    seq: Some(WordInfoSeq::Single(1298520)),
                    counter: Some(("Value: 3".to_string(), false)),
                    ..Default::default()
                },
                "三冊 【さんさつ】\nValue: 3\n1. [ctr] counter for books\n2. [n] volume",
            ),
            (
                "F",
                WordInfo {
                    kind: Kanji,
                    text: "三".to_string(),
                    kana: single("さん"),
                    seq: None,
                    counter: Some(("Value: 3".to_string(), false)),
                    ..Default::default()
                },
                "三 【さん】\nValue: 3",
            ),
            (
                "G",
                compound(
                    "食べたい",
                    "たべたい",
                    &[1358280, 2017560],
                    vec![
                        WordInfo {
                            kind: Kanji,
                            text: "食べ".to_string(),
                            kana: single("たべ"),
                            seq: Some(WordInfoSeq::Single(1358280)),
                            primary: true,
                            ..Default::default()
                        },
                        WordInfo {
                            kind: Kana,
                            text: "たい".to_string(),
                            kana: single("たい"),
                            seq: Some(WordInfoSeq::Single(2017560)),
                            primary: false,
                            ..Default::default()
                        },
                    ],
                ),
                "食べたい 【たべたい】 Compound word: 食べ + たい\n * 食べ 【たべ】\n1. [v1,vt] to eat\n2. [vt,v1] to live on (e.g. a salary); to live off; to subsist on\n * たい  [suffix]: want to... / would like to... ",
            ),
            (
                "G2",
                compound(
                    "日本語",
                    "にほんご",
                    &[1582710, 1576050],
                    vec![
                        WordInfo {
                            kind: Kanji,
                            text: "日本".to_string(),
                            kana: single("にほん"),
                            seq: Some(WordInfoSeq::Single(1582710)),
                            primary: true,
                            ..Default::default()
                        },
                        WordInfo {
                            kind: Kanji,
                            text: "語".to_string(),
                            kana: single("ご"),
                            seq: Some(WordInfoSeq::Single(1576050)),
                            primary: false,
                            ..Default::default()
                        },
                    ],
                ),
                "日本語 【にほんご】 Compound word: 日本 + 語\n * 日本 【にほん】\n1. [n] Japan\n * 語 【ご】\n1. [adv,n] day before yesterday",
            ),
            (
                "H",
                WordInfo {
                    kind: Kanji,
                    text: "一人".to_string(),
                    kana: single("ひとり"),
                    seq: Some(WordInfoSeq::Multi(vec![
                        Some(WordInfoSeq::Single(1576150)),
                        Some(WordInfoSeq::Single(2149890)),
                    ])),
                    alternative: true,
                    components: vec![
                        WordInfo {
                            kind: Kanji,
                            text: "一人".to_string(),
                            kana: single("ひとり"),
                            seq: Some(WordInfoSeq::Single(1576150)),
                            primary: false,
                            ..Default::default()
                        },
                        WordInfo {
                            kind: Kanji,
                            text: "一人".to_string(),
                            kana: single("ひとり"),
                            seq: Some(WordInfoSeq::Single(2149890)),
                            counter: Some(("Value: 1".to_string(), false)),
                            primary: false,
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
                "<1>. 一人 【ひとり】\n1. [n] 《esp. 一人, １人》 one person\n2. [n] being alone; being by oneself\n3. [n] 《esp. 独り》 being single; being unmarried\n4. [adv] by oneself; alone\n5. [adv] 《with neg. sentence》 just; only; simply\n<2>. 一人 【ひとり】\nValue: 1\n1. [ctr] counter for people",
            ),
        ];

        for (label, word_info, expected) in &cases {
            assert_eq!(
                &word_info_str(&ctx, word_info).await.unwrap(),
                expected,
                "case={label}"
            );
        }
    }
}

mod word_info_gloss_json {
    use crate::dict::find_word_info::find_word_info;
    use crate::dict::word_info_str::*;
    // Ground truth captured from `(jsown:to-json (word-info-gloss-json …))`
    // and `find-word-info-json` on .103 (2026-05-25) after `(init-suffixes t t)`.
    // jsown emits `\uXXXX`; the expected strings below are the
    // raw-UTF-8 round-trip serde_json produces (identical JSON). Tests run
    // against the local DB per project policy.

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn json(value: &Value) -> String {
        serde_json::to_string(value).unwrap()
    }

    /// Drives word-info-gloss-json through find-word-info (non-root). One row
    /// per cond branch: simple noun (`(t)` arm, top-level gloss + has-gloss →
    /// empty conj), conjugated verb (conjs is a list → no top-level gloss; conj
    /// carries it), ordinal counter (counter object, `ordinal:true`), and
    /// compound (compound list + recursive components, the non-primary いる
    /// reaching the suffix arm).
    #[tokio::test]
    async fn word_info_gloss_json_branches() {
        let ctx = ctx_from_env().await;
        // (text, expected single-object json)
        let cases: &[(&str, &str)] = &[
            // simple noun — no conjs → top-level gloss, has-gloss → empty conj.
            (
                "政府",
                r#"{"reading":"政府 【せいふ】","text":"政府","kana":"せいふ","score":325,"seq":1376070,"gloss":[{"pos":"[n]","gloss":"government; administration; ministry"}],"conj":[]}"#,
            ),
            // conjugated verb — conjs is a list (not nil/:root) → no top-level
            // gloss; conj-info-json (has-gloss nil) carries the gloss.
            (
                "書いた",
                r#"{"reading":"書いた 【かいた】","text":"書いた","kana":"かいた","score":336,"seq":10526928,"conj":[{"prop":[{"pos":"v5k","type":"Past (~ta)"}],"reading":"書く 【かく】","gloss":[{"pos":"[v5k,vt]","gloss":"to write; to compose; to pen"},{"pos":"[vt,v5k]","gloss":"to draw; to paint"}],"readok":true}]}"#,
            ),
            // ordinal counter — counter object with ordinal:true.
            (
                "5番目",
                r#"{"reading":"5番目 【ごばんめ】","text":"5番目","kana":"ごばんめ","score":667,"counter":{"value":"Value: 5th","ordinal":true},"seq":1482410,"gloss":[{"pos":"[ctr]","gloss":"the nth ...","info":"indicates position in a sequence"}]}"#,
            ),
            // compound — compound list + components, the non-primary いる
            // component reaches the suffix arm (suffix t, has a description).
            (
                "食べてる",
                r#"{"reading":"食べてる 【たべてる】","text":"食べてる","kana":"たべてる","score":434,"compound":["食べて","いる"],"components":[{"reading":"食べて 【たべて】","text":"食べて","kana":"たべて","score":0,"seq":10092233,"conj":[{"prop":[{"pos":"v1","type":"Conjunctive (~te)"}],"reading":"食べる 【たべる】","gloss":[{"pos":"[v1,vt]","gloss":"to eat"},{"pos":"[vt,v1]","gloss":"to live on (e.g. a salary); to live off; to subsist on"}],"readok":true}]},{"reading":"いる","text":"いる","kana":"いる","score":0,"seq":1577980,"suffix":"indicates continuing action (to be ...ing)","conj":[]}]}"#,
            ),
        ];
        for (text, expected) in cases {
            let wis = find_word_info(&ctx, text, None, false).await.unwrap();
            assert!(!wis.is_empty(), "text={text}");
            let result = word_info_gloss_json(&ctx, &wis[0], false).await.unwrap();
            assert_eq!(json(&result), *expected, "text={text}");
        }
    }

    /// `root_only=true` on a non-root word-info: the `(t)` arm adds gloss
    /// unconditionally and returns before the conj key. 政府 yields a
    /// populated gloss; 書いた's conjugated seq has no direct senses, so the
    /// gloss is the empty list (still emitted, unlike the guarded non-root arm).
    #[tokio::test]
    async fn root_only_t_arm() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, &str)] = &[
            (
                "政府",
                r#"{"reading":"政府 【せいふ】","text":"政府","kana":"せいふ","score":325,"seq":1376070,"gloss":[{"pos":"[n]","gloss":"government; administration; ministry"}]}"#,
            ),
            (
                "書いた",
                r#"{"reading":"書いた 【かいた】","text":"書いた","kana":"かいた","score":336,"seq":10526928,"gloss":[]}"#,
            ),
        ];
        for (text, expected) in cases {
            let wis = find_word_info(&ctx, text, None, false).await.unwrap();
            assert!(!wis.is_empty(), "text={text}");
            let result = word_info_gloss_json(&ctx, &wis[0], true).await.unwrap();
            assert_eq!(json(&result), *expected, "text={text}");
        }
    }

    /// 5個 → two counter readings (ごこ / ごか); each carries a non-ordinal
    /// counter object (`ordinal:[]`) and a pos-list `("ctr")`-filtered gloss.
    #[tokio::test]
    async fn counter_two_readings() {
        let ctx = ctx_from_env().await;
        let expected = [
            r#"{"reading":"5個 【ごこ】","text":"5個","kana":"ごこ","score":128,"counter":{"value":"Value: 5","ordinal":[]},"seq":1264740,"gloss":[{"pos":"[ctr]","gloss":"counter for (small) things or pieces","info":"also written as ヶ"},{"pos":"[ctr]","gloss":"counter for military units"}]}"#,
            r#"{"reading":"5個 【ごか】","text":"5個","kana":"ごか","score":40,"counter":{"value":"Value: 5","ordinal":[]},"seq":2220320,"gloss":[{"pos":"[ctr]","gloss":"counter used with Sino-Japanese words","info":"precedes the thing being counted; e.g. ３か月, ５か所; also written as ヶ or ケ"}]}"#,
        ];
        let wis = find_word_info(&ctx, "5個", None, false).await.unwrap();
        assert_eq!(wis.len(), 2);
        for (wi, expected) in wis.iter().zip(expected.iter()) {
            let result = word_info_gloss_json(&ctx, wi, false).await.unwrap();
            assert_eq!(json(&result), *expected);
        }
    }

    /// Alternative branch: `{"alternative": [word_info_gloss_json_inner(c) …]}`. Built from a real
    /// alternative word-info (the segmenter shape: alternative=t, components =
    /// the surviving word-infos). find-word-info never produces alternatives,
    /// so the components are assembled from its 何 (なに / なん) word-infos.
    #[tokio::test]
    async fn alternative_branch() {
        let ctx = ctx_from_env().await;
        let components = find_word_info(&ctx, "何", None, false).await.unwrap();
        assert_eq!(components.len(), 2);
        let alt = WordInfo {
            kind: crate::dict::word_info::WordInfoType::Kanji,
            text: "何".to_owned(),
            alternative: true,
            components,
            start: Some(0),
            end: Some(1),
            ..WordInfo::default()
        };
        let result = word_info_gloss_json(&ctx, &alt, false).await.unwrap();
        let expected = r#"{"alternative":[{"reading":"何 【なに】","text":"何","kana":"なに","score":24,"seq":1577100,"gloss":[{"pos":"[pn]","gloss":"what"},{"pos":"[pn]","gloss":"you-know-what; that thing"},{"pos":"[pn]","gloss":"whatsit; whachamacallit; what's-his-name; what's-her-name"},{"pos":"[n]","gloss":"penis; (one's) thing; dick","info":"esp. ナニ"},{"pos":"[adv]","gloss":"(not) at all; (not) in the slightest","info":"with neg. sentence"},{"pos":"[int]","gloss":"what?; huh?","info":"indicates surprise"},{"pos":"[int]","gloss":"hey!; come on!","info":"indicates anger or irritability"},{"pos":"[int]","gloss":"oh, no (it's fine); why (it's nothing); oh (certainly not)","info":"used to dismiss someone's worries, concerns, etc."}],"conj":[]},{"reading":"何 【なん】","text":"何","kana":"なん","score":16,"seq":2846738,"gloss":[{"pos":"[pn]","gloss":"what"},{"pos":"[pref]","gloss":"how many","info":"followed by a counter"},{"pos":"[pref]","gloss":"many; a lot of","info":"followed by (optional number), counter and も"},{"pos":"[pref]","gloss":"several; a few; some","info":"followed by a counter and か"}],"conj":[]}]}"#;
        assert_eq!(json(&result), expected);
    }
}

mod get_kanji_words {
    use crate::dict::word_info_str::*;
    // Every assertion is REPL-verified against the .103 SBCL via
    // `(ichiran/dict::get-kanji-words …)` (2026-05-25 probe).
    // Run with `-- --test-threads=1` per the DB-test convention.

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    fn row(seq: i32, kanji: &str, kana: &str, common: i32) -> (i32, String, String, i32) {
        (seq, kanji.to_string(), kana.to_string(), common)
    }

    /// The query has no ORDER BY, so the result is an unordered set; both
    /// sides are sorted by seq before comparison. `蜂蜜` carries
    /// `common = 0`, exercising the non-null-but-zero branch of the
    /// `(:not-null 'k.common)` filter.
    #[tokio::test]
    async fn get_kanji_words_fixtures() {
        let ctx = ctx().await;
        let cases: &[(&str, Vec<(i32, String, String, i32)>)] = &[
            (
                "鯨",
                vec![
                    row(1253270, "鯨", "くじら", 13),
                    row(1253290, "鯨肉", "げいにく", 30),
                    row(1514180, "捕鯨", "ほげい", 6),
                ],
            ),
            ("錐", vec![row(1175930, "円錐", "まるぎり", 44)]),
            (
                "蜂",
                vec![
                    row(1517840, "蜂", "はち", 34),
                    row(1517860, "蜂蜜", "はちみつ", 0),
                    row(1729030, "蜂起", "ほうき", 39),
                ],
            ),
        ];
        for (char, expected) in cases {
            let mut got = get_kanji_words(&ctx, char).await.unwrap();
            // Result is an unordered set (no ORDER BY); sort the whole
            // tuple so the comparison is deterministic even if a char
            // ever yields two rows sharing a seq.
            got.sort();
            let mut expected = expected.clone();
            expected.sort();
            assert_eq!(got, expected, "char={char:?}");
        }
    }

    /// `#\火` (char) and `"火"` (string) hit the same query in upstream;
    /// the Rust port collapses both to `&str`, so a single-character
    /// argument returns the full substring match set.
    #[tokio::test]
    async fn single_char_argument() {
        let ctx = ctx().await;
        let words = get_kanji_words(&ctx, "火").await.unwrap();
        assert_eq!(words.len(), 75);
    }
}
