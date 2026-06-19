mod reading_str_star_ {
    use crate::dict::word_info_str::*;

    /// Covers kanji+kana, kana-only (no kanji → bare kana), kanji with no
    /// kana (renders "NIL"), and both absent (→ None).
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

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// Covers a kanji+kana entry, a conjugating kanji+kana verb, two
    /// kana-only entries (no kanji → bare kana), and an unknown seq (→ None).
    /// Needs a live Postgres DB.
    #[test]
    fn reading_str_seq_fixtures() {
        let ctx = ctx_from_env();
        let cases: &[(i32, Option<&str>)] = &[
            (1582710, Some("日本 【にほん】")),
            (1358280, Some("食べる 【たべる】")),
            (1010890, Some("ぴちぴち")),
            (1056070, Some("サイバネーション")),
            (999999, None),
        ];
        for (seq, expected) in cases {
            assert_eq!(
                reading_str_seq(&ctx, *seq).unwrap().as_deref(),
                *expected,
                "seq={seq}"
            );
        }
    }
}

mod entry_info_short {
    use crate::dict::word_info_str::*;

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// Covers a noun (日本) and a verb (食べる) with no part-of-speech filter,
    /// a filter matching the entry's part of speech, a filter matching no
    /// sense (empty after " : "), and an unknown seq (reading renders "NIL").
    /// Needs a live Postgres DB.
    #[test]
    fn entry_info_short_fixtures() {
        let ctx = ctx_from_env();
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
                entry_info_short(&ctx, *seq, *with_pos).unwrap(),
                *expected,
                "seq={seq} with_pos={with_pos:?}"
            );
        }
    }
}

mod entry_info_long {
    use crate::dict::word_info_str::*;

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// Covers a single-sense noun (reading with 【kanji】), a verb, a
    /// kana-only entry (bare kana, no 【】), a multi-sense entry with a
    /// 《sense-info》 annotation, and an unknown seq (just the seq, no reading).
    /// Needs a live Postgres DB.
    #[test]
    fn entry_info_long_fixtures() {
        let ctx = ctx_from_env();
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
                &entry_info_long(&ctx, *seq).unwrap(),
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

    /// Uppercases a kana element's text.
    fn upcase(element: &Option<WordInfoKana>) -> String {
        match element {
            Some(WordInfoKana::Single(text)) => text.to_uppercase(),
            other => format!("{other:?}"),
        }
    }

    /// Returns a kana element's text unchanged.
    fn ident(element: &Option<WordInfoKana>) -> String {
        match element {
            Some(WordInfoKana::Single(text)) => text.clone(),
            other => format!("{other:?}"),
        }
    }

    #[test]
    fn map_word_info_kana_fixtures() {
        // Single reading: the function is applied to the one kana value.
        assert_eq!(map_word_info_kana(upcase, &wi(single("neko")), "/"), "NEKO");

        // Multiple readings, default separator "/".
        let inu = Some(WordInfoKana::Multi(vec![single("neko"), single("inu")]));
        assert_eq!(
            map_word_info_kana(upcase, &wi(inu.clone()), "/"),
            "NEKO/INU"
        );

        // Two readings that share a de-spaced form merge into one, joined by
        // a middle-dot.
        let merge = Some(WordInfoKana::Multi(vec![single("a b"), single("ab")]));
        assert_eq!(map_word_info_kana(ident, &wi(merge), "/"), "a\u{00B7}b");

        // No kana → empty string.
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

    /// Covers the KANJI type (single reading / reading list / nested list with
    /// a nil / no kana rendered "NIL"), a counter-with-seq reaching the same
    /// rendering, and the plain branch (counter without seq, KANA type, GAP type).
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

    fn ctx_from_env() -> Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    fn single(reading: &str) -> Option<WordInfoKana> {
        Some(WordInfoKana::Single(reading.to_string()))
    }

    /// Each row builds one word-info and pins its exact rendered string
    /// (blank lines included). Needs a live Postgres DB. Cases:
    /// - A: plain word, no conjugations → senses.
    /// - B: conjugated word → empty senses plus full conjugation info.
    /// - C: root form → conjugation display suppressed.
    /// - D: no seq → "???".
    /// - E: counter with seq → value then senses.
    /// - F: counter, no seq → value only.
    /// - G: compound whose non-primary part is a suffix → marker, suffix description.
    /// - G2: compound whose non-primary part is not a suffix → marker, then senses.
    /// - H: alternative → "<i>. " prefixes, second reading a counter.
    #[test]
    fn word_info_str_fixtures() {
        use WordInfoType::{Kana, Kanji};
        let ctx = ctx_from_env();

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

        // Cases B/C feed a conjugated-form entry seq (past of 食べる) that
        // renumbers per build; resolve it from the stable surface 食べた.
        let tabeta = crate::test_support::conj_entry_seq("食べた");
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
                    seq: Some(WordInfoSeq::Single(tabeta)),
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
                    seq: Some(WordInfoSeq::Single(tabeta)),
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
                &word_info_str(&ctx, word_info).unwrap(),
                expected,
                "case={label}"
            );
        }
    }
}

mod word_info_gloss_json {
    use crate::dict::find_word_info::find_word_info;
    use crate::dict::word_info_str::*;
    // Needs a live Postgres DB.

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    fn json(value: &Value) -> String {
        serde_json::to_string(value).unwrap()
    }

    /// One case per output shape: simple noun (top-level gloss, empty conj),
    /// conjugated verb (no top-level gloss; the conjugation carries it),
    /// ordinal counter (counter object, `ordinal:true`), and compound (with
    /// recursive components, the non-primary いる rendered as a suffix).
    #[test]
    fn word_info_gloss_json_branches() {
        let ctx = ctx_from_env();
        // (text, expected single-object json)
        let cases: &[(&str, &str)] = &[
            // 政府 — simple noun: top-level gloss, empty conj.
            (
                "政府",
                r#"{"reading":"政府 【せいふ】","text":"政府","kana":"せいふ","score":325,"seq":1376070,"gloss":[{"pos":"[n]","gloss":"government; administration; ministry"}],"conj":[]}"#,
            ),
            // 書いた — conjugated verb: no top-level gloss; the conjugation carries it.
            (
                "書いた",
                r#"{"reading":"書いた 【かいた】","text":"書いた","kana":"かいた","score":336,"seq":10526928,"conj":[{"prop":[{"pos":"v5k","type":"Past (~ta)"}],"reading":"書く 【かく】","gloss":[{"pos":"[v5k,vt]","gloss":"to write; to compose; to pen"},{"pos":"[vt,v5k]","gloss":"to draw; to paint"}],"readok":true}]}"#,
            ),
            // 5番目 — ordinal counter: counter object with ordinal:true.
            (
                "5番目",
                r#"{"reading":"5番目 【ごばんめ】","text":"5番目","kana":"ごばんめ","score":667,"counter":{"value":"Value: 5th","ordinal":true},"seq":1482410,"gloss":[{"pos":"[ctr]","gloss":"the nth ...","info":"indicates position in a sequence"}]}"#,
            ),
            // 食べてる — compound: components, the non-primary いる rendered as a suffix.
            (
                "食べてる",
                r#"{"reading":"食べてる 【たべてる】","text":"食べてる","kana":"たべてる","score":434,"compound":["食べて","いる"],"components":[{"reading":"食べて 【たべて】","text":"食べて","kana":"たべて","score":0,"seq":10092233,"conj":[{"prop":[{"pos":"v1","type":"Conjunctive (~te)"}],"reading":"食べる 【たべる】","gloss":[{"pos":"[v1,vt]","gloss":"to eat"},{"pos":"[vt,v1]","gloss":"to live on (e.g. a salary); to live off; to subsist on"}],"readok":true}]},{"reading":"いる","text":"いる","kana":"いる","score":0,"seq":1577980,"suffix":"indicates continuing action (to be ...ing)","conj":[]}]}"#,
            ),
        ];
        for (text, expected) in cases {
            let wis = find_word_info(&ctx, text, None, false).unwrap();
            assert!(!wis.is_empty(), "text={text}");
            let result = word_info_gloss_json(&ctx, &wis[0], false).unwrap();
            // 書いた / 食べてる carry synthetic conjugated-entry seqs that
            // renumber per build; compare ignoring those.
            crate::test_support::assert_json_seq_agnostic(&result, expected, text);
        }
    }

    /// With root_only=true, gloss is always emitted and there's no conj key.
    /// 政府 yields a populated gloss; 書いた has no direct senses, so its gloss
    /// is the empty list (still present).
    #[test]
    fn root_only_t_arm() {
        let ctx = ctx_from_env();
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
            let wis = find_word_info(&ctx, text, None, false).unwrap();
            assert!(!wis.is_empty(), "text={text}");
            let result = word_info_gloss_json(&ctx, &wis[0], true).unwrap();
            // 書いた carries a synthetic conjugated-entry seq that renumbers
            // per build; compare ignoring those.
            crate::test_support::assert_json_seq_agnostic(&result, expected, text);
        }
    }

    /// An alternative word-info serializes to {"alternative": [...]} with one
    /// object per component. Built from 何's two readings (なに / なん).
    #[test]
    fn alternative_branch() {
        let ctx = ctx_from_env();
        let components = find_word_info(&ctx, "何", None, false).unwrap();
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
        let result = word_info_gloss_json(&ctx, &alt, false).unwrap();
        let expected = r#"{"alternative":[{"reading":"何 【なに】","text":"何","kana":"なに","score":24,"seq":1577100,"gloss":[{"pos":"[pn]","gloss":"what"},{"pos":"[pn]","gloss":"you-know-what; that thing"},{"pos":"[pn]","gloss":"whatsit; whachamacallit; what's-his-name; what's-her-name"},{"pos":"[n]","gloss":"penis; (one's) thing; dick","info":"esp. ナニ"},{"pos":"[adv]","gloss":"(not) at all; (not) in the slightest","info":"with neg. sentence"},{"pos":"[int]","gloss":"what?; huh?","info":"indicates surprise"},{"pos":"[int]","gloss":"hey!; come on!","info":"indicates anger or irritability"},{"pos":"[int]","gloss":"oh, no (it's fine); why (it's nothing); oh (certainly not)","info":"used to dismiss someone's worries, concerns, etc."}],"conj":[]},{"reading":"何 【なん】","text":"何","kana":"なん","score":16,"seq":2846738,"gloss":[{"pos":"[pn]","gloss":"what"},{"pos":"[pref]","gloss":"how many","info":"followed by a counter"},{"pos":"[pref]","gloss":"many; a lot of","info":"followed by (optional number), counter and も"},{"pos":"[pref]","gloss":"several; a few; some","info":"followed by a counter and か"}],"conj":[]}]}"#;
        assert_eq!(json(&result), expected);
    }
}

mod get_kanji_words {
    use crate::dict::word_info_str::*;
    // Needs a live Postgres DB; run single-threaded (`-- --test-threads=1`).

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    fn row(seq: i32, kanji: &str, kana: &str, common: i32) -> (i32, String, String, i32) {
        (seq, kanji.to_string(), kana.to_string(), common)
    }

    /// The result is an unordered set; both sides are sorted before comparison.
    /// 蜂蜜 carries common = 0, exercising the common-is-zero-but-not-null case.
    #[test]
    fn get_kanji_words_fixtures() {
        let ctx = ctx();
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
            let mut got = get_kanji_words(&ctx, char).unwrap();
            // The result is unordered; sort both sides for a stable comparison.
            got.sort();
            let mut expected = expected.clone();
            expected.sort();
            assert_eq!(got, expected, "char={char:?}");
        }
    }

    /// A single-character argument ("火") returns the full set of words
    /// containing that kanji.
    #[test]
    fn single_char_argument() {
        let ctx = ctx();
        let words = get_kanji_words(&ctx, "火").unwrap();
        assert_eq!(words.len(), 75);
    }
}
