mod word_info_json {
    use crate::dict::word_info::*;

    fn single_kana(s: &str) -> Option<WordInfoKana> {
        Some(WordInfoKana::Single(s.to_owned()))
    }

    /// Serializes word-info values to JSON across the main shapes.
    #[test]
    fn word_info_json_fixtures() {
        // 食べた — plain kanji word: single kana/seq, no conjugations/counter,
        // primary true.
        let tabeta = WordInfo {
            kind: WordInfoType::Kanji,
            text: "食べた".to_owned(),
            true_text: Some("食べた".to_owned()),
            kana: single_kana("たべた"),
            seq: Some(WordInfoSeq::Single(10092229)),
            score: Some(336),
            start: Some(0),
            end: Some(3),
            ..WordInfo::default()
        };
        assert_eq!(
            serde_json::to_string(&word_info_json(&tabeta)).unwrap(),
            r#"{"type":"KANJI","text":"食べた","truetext":"食べた","kana":"たべた","seq":10092229,"conjugations":[],"score":336,"components":[],"alternative":[],"primary":true,"start":0,"end":3,"counter":[],"skipped":0}"#
        );

        // 5番目 — ordinal counter: counter array [value-string, true], empty truetext.
        let go_banme = WordInfo {
            kind: WordInfoType::Kanji,
            text: "5番目".to_owned(),
            true_text: None,
            kana: single_kana("ごばんめ"),
            seq: Some(WordInfoSeq::Single(1482410)),
            score: Some(667),
            start: Some(0),
            end: Some(3),
            counter: Some(("Value: 5th".to_owned(), true)),
            ..WordInfo::default()
        };
        assert_eq!(
            serde_json::to_string(&word_info_json(&go_banme)).unwrap(),
            r#"{"type":"KANJI","text":"5番目","truetext":[],"kana":"ごばんめ","seq":1482410,"conjugations":[],"score":667,"components":[],"alternative":[],"primary":true,"start":0,"end":3,"counter":["Value: 5th",true],"skipped":0}"#
        );

        // 走っている — compound: multi-valued seq, nested components, conjugations
        // as an id list (走って) and the root marker (いる, non-primary),
        // empty component start/end.
        let hashitteiru = WordInfo {
            kind: WordInfoType::Kanji,
            text: "走っている".to_owned(),
            true_text: None,
            kana: single_kana("はしっている"),
            seq: Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(10063379)),
                Some(WordInfoSeq::Single(1577980)),
            ])),
            score: Some(406),
            components: vec![
                WordInfo {
                    kind: WordInfoType::Kanji,
                    text: "走って".to_owned(),
                    true_text: Some("走って".to_owned()),
                    kana: single_kana("はしって"),
                    seq: Some(WordInfoSeq::Single(10063379)),
                    conjugations: Some(WordConjugations::Ids(vec![63591])),
                    score: Some(0),
                    start: None,
                    end: None,
                    ..WordInfo::default()
                },
                WordInfo {
                    kind: WordInfoType::Kana,
                    text: "いる".to_owned(),
                    true_text: Some("いる".to_owned()),
                    kana: single_kana("いる"),
                    seq: Some(WordInfoSeq::Single(1577980)),
                    conjugations: Some(WordConjugations::Root),
                    score: Some(0),
                    primary: false,
                    start: None,
                    end: None,
                    ..WordInfo::default()
                },
            ],
            start: Some(0),
            end: Some(5),
            ..WordInfo::default()
        };
        assert_eq!(
            serde_json::to_string(&word_info_json(&hashitteiru)).unwrap(),
            r#"{"type":"KANJI","text":"走っている","truetext":[],"kana":"はしっている","seq":[10063379,1577980],"conjugations":[],"score":406,"components":[{"type":"KANJI","text":"走って","truetext":"走って","kana":"はしって","seq":10063379,"conjugations":[63591],"score":0,"components":[],"alternative":[],"primary":true,"start":[],"end":[],"counter":[],"skipped":0},{"type":"KANA","text":"いる","truetext":"いる","kana":"いる","seq":1577980,"conjugations":"ROOT","score":0,"components":[],"alternative":[],"primary":[],"start":[],"end":[],"counter":[],"skipped":0}],"alternative":[],"primary":true,"start":0,"end":5,"counter":[],"skipped":0}"#
        );

        // 何 — alternative branch: multiple kana, multiple seq, alternative true,
        // two components.
        let nani = WordInfo {
            kind: WordInfoType::Kanji,
            text: "何".to_owned(),
            true_text: None,
            kana: Some(WordInfoKana::Multi(vec![
                Some(WordInfoKana::Single("なに".to_owned())),
                Some(WordInfoKana::Single("なん".to_owned())),
            ])),
            seq: Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(1577100)),
                Some(WordInfoSeq::Single(2846738)),
            ])),
            score: Some(24),
            components: vec![
                WordInfo {
                    kind: WordInfoType::Kanji,
                    text: "何".to_owned(),
                    true_text: Some("何".to_owned()),
                    kana: single_kana("なに"),
                    seq: Some(WordInfoSeq::Single(1577100)),
                    score: Some(24),
                    start: Some(0),
                    end: Some(1),
                    ..WordInfo::default()
                },
                WordInfo {
                    kind: WordInfoType::Kanji,
                    text: "何".to_owned(),
                    true_text: Some("何".to_owned()),
                    kana: single_kana("なん"),
                    seq: Some(WordInfoSeq::Single(2846738)),
                    score: Some(16),
                    start: Some(0),
                    end: Some(1),
                    ..WordInfo::default()
                },
            ],
            alternative: true,
            start: Some(0),
            end: Some(1),
            ..WordInfo::default()
        };
        assert_eq!(
            serde_json::to_string(&word_info_json(&nani)).unwrap(),
            r#"{"type":"KANJI","text":"何","truetext":[],"kana":["なに","なん"],"seq":[1577100,2846738],"conjugations":[],"score":24,"components":[{"type":"KANJI","text":"何","truetext":"何","kana":"なに","seq":1577100,"conjugations":[],"score":24,"components":[],"alternative":[],"primary":true,"start":0,"end":1,"counter":[],"skipped":0},{"type":"KANJI","text":"何","truetext":"何","kana":"なん","seq":2846738,"conjugations":[],"score":16,"components":[],"alternative":[],"primary":true,"start":0,"end":1,"counter":[],"skipped":0}],"alternative":true,"primary":true,"start":0,"end":1,"counter":[],"skipped":0}"#
        );
    }
}

mod simple_word_info {
    use crate::dict::word_info::*;

    /// The JSON output form returns the serialized word-info.
    #[test]
    fn simple_word_info_json() {
        let out = simple_word_info(
            Some(WordInfoSeq::Single(1234567)),
            "テスト",
            Some(WordInfoKana::Single("てすと".to_owned())),
            WordInfoType::Kana,
            SimpleWordInfoAs::Json,
        );
        let KaniSimpleWordInfo::Json(v) = out else {
            panic!("expected Json variant");
        };
        assert_eq!(
            serde_json::to_string(&v).unwrap(),
            r#"{"type":"KANA","text":"テスト","truetext":"テスト","kana":"てすと","seq":1234567,"conjugations":[],"score":0,"components":[],"alternative":[],"primary":true,"start":[],"end":[],"counter":[],"skipped":0}"#
        );
    }

    /// The object output form (the default) returns the constructed word-info;
    /// true-text mirrors text and unset fields take their defaults.
    #[test]
    fn simple_word_info_object() {
        let out = simple_word_info(
            Some(WordInfoSeq::Single(10092229)),
            "食べた",
            Some(WordInfoKana::Single("たべた".to_owned())),
            WordInfoType::Kanji,
            SimpleWordInfoAs::Object,
        );
        let KaniSimpleWordInfo::Object(wi) = out else {
            panic!("expected Object variant");
        };
        assert_eq!(wi.text, "食べた");
        assert_eq!(wi.true_text.as_deref(), Some("食べた"));
        assert_eq!(wi.kind, WordInfoType::Kanji);
        assert_eq!(wi.score, Some(0));
        assert!(wi.primary);
    }
}

mod def_reader_for_json_macro {
    use crate::dict::word_info::*;
    use serde_json::json;

    #[test]
    fn reads_each_slot() {
        // 薔薇 — single-seq noun: scalar slots, every empty slot serialized as [].
        let bara = json!({
            "type":"KANJI","text":"薔薇","truetext":"薔薇","kana":"ばら",
            "seq":1571760,"conjugations":[],"score":143,"components":[],
            "alternative":[],"primary":true,"start":0,"end":2,"counter":[],"skipped":0
        });
        let cases: &[(&str, Value)] = &[
            ("text", json!("薔薇")),
            ("truetext", json!("薔薇")),
            ("kana", json!("ばら")),
            ("seq", json!(1571760)),
            ("score", json!(143)),
            ("components", json!([])),
            ("alternative", json!([])),
            ("primary", json!(true)),
            ("start", json!(0)),
            ("end", json!(2)),
            ("counter", json!([])),
            ("skipped", json!(0)),
        ];
        for &(slot, ref expected) in cases {
            assert_eq!(def_reader_for_json(&bara, slot), expected, "slot={slot}");
        }
    }

    #[test]
    fn reads_multi_value_slots() {
        // 一人 — alternative reading: kana/seq are arrays, components is the
        // two-child array (counter on the second child), alternative true,
        // empty truetext.
        let hitori = json!({
            "type":"KANJI","text":"一人","truetext":[],"kana":["ひとり"],
            "seq":[1576150,2149890],"conjugations":[],"score":312,
            "components":[
                {"type":"KANJI","text":"一人","truetext":"一人","kana":"ひとり","seq":1576150,
                 "conjugations":[],"score":312,"components":[],"alternative":[],"primary":true,
                 "start":0,"end":2,"counter":[],"skipped":0},
                {"type":"KANJI","text":"一人","truetext":[],"kana":"ひとり","seq":2149890,
                 "conjugations":[],"score":208,"components":[],"alternative":[],"primary":true,
                 "start":0,"end":2,"counter":["Value: 1",[]],"skipped":0}
            ],
            "alternative":true,"primary":true,"start":0,"end":2,"counter":[],"skipped":0
        });
        assert_eq!(def_reader_for_json(&hitori, "kana"), &json!(["ひとり"]));
        assert_eq!(
            def_reader_for_json(&hitori, "seq"),
            &json!([1576150, 2149890])
        );
        assert_eq!(def_reader_for_json(&hitori, "alternative"), &json!(true));
        assert_eq!(def_reader_for_json(&hitori, "truetext"), &json!([]));
        let components = def_reader_for_json(&hitori, "components");
        let second_child = &components.as_array().expect("components is an array")[1];
        assert_eq!(
            def_reader_for_json(second_child, "counter"),
            &json!(["Value: 1", []])
        );
    }

    #[test]
    #[should_panic(expected = "not present")]
    fn panics_on_missing_key() {
        let obj = json!({"text": "x"});
        def_reader_for_json(&obj, "nonexistent");
    }
}

mod word_info_from_segment {
    use crate::dict::counters::dispatchers::find_counter;
    use crate::dict::readings::{find_word, FindWordRows};
    use crate::dict::word_info::*;
    // Needs a live Postgres DB.

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn first_reading(ctx: &KaniranContext, word: &str) -> KaniWordDispatchEnum {
        let rows = find_word(ctx, word, false).await.unwrap().into_owned();
        match rows {
            FindWordRows::Kanji(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kanji)
                .unwrap_or_else(|| panic!("no kanji rows for {word:?}")),
            FindWordRows::Kana(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kana)
                .unwrap_or_else(|| panic!("no kana rows for {word:?}")),
        }
    }

    fn segment(word: KaniWordDispatchEnum, score: i32, start: usize, end: usize) -> Segment {
        Segment {
            start,
            end,
            word,
            score: Some(score),
            info: None,
            top: None,
            text: None,
        }
    }

    #[tokio::test]
    async fn kana_text_segment_populates_simple_text_slots() {
        // A kana word (ねこ) yields a KANA word-info with text/kana/true-text
        // all ねこ, primary set, and no counter.
        let ctx = ctx_from_env().await;
        let word = first_reading(&ctx, "ねこ").await;
        let mut seg = segment(word, 16, 0, 2);
        let wi = word_info_from_segment(&ctx, &mut seg).await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kana);
        assert_eq!(wi.text, "ねこ");
        assert_eq!(wi.kana, Some(WordInfoKana::Single("ねこ".into())));
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(1467640)));
        assert_eq!(wi.score, Some(16));
        assert_eq!(wi.start, Some(0));
        assert_eq!(wi.end, Some(2));
        assert_eq!(wi.true_text.as_deref(), Some("ねこ"));
        assert!(wi.counter.is_none());
        assert!(wi.primary);
        assert!(!wi.alternative);
        assert_eq!(wi.skipped, 0);
        assert!(wi.components.is_empty());
    }

    #[tokio::test]
    async fn kanji_text_segment_returns_text_and_seq() {
        let ctx = ctx_from_env().await;
        let word = first_reading(&ctx, "猫").await;
        let mut seg = segment(word, 3, 0, 1);
        let wi = word_info_from_segment(&ctx, &mut seg).await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kanji);
        assert_eq!(wi.text, "猫");
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(2698030)));
        assert_eq!(wi.score, Some(3));
        assert_eq!(wi.true_text.as_deref(), Some("猫"));
        assert!(wi.counter.is_none());
        assert!(matches!(wi.kana, Some(WordInfoKana::Single(_))));
    }

    #[tokio::test]
    async fn counter_text_segment_populates_counter_pair_and_null_true_text() {
        // A counter word (5個) yields a KANJI word-info with a counter pair
        // and no true-text.
        let ctx = ctx_from_env().await;
        let counter = find_counter(&ctx, "5", "個", None)
            .into_iter()
            .next()
            .expect("find_counter(5, 個) returned no counters");
        let word = KaniWordDispatchEnum::Counter(counter);
        let mut seg = segment(word, 40, 0, 2);
        let wi = word_info_from_segment(&ctx, &mut seg).await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kanji);
        assert_eq!(wi.text, "5個");
        assert_eq!(wi.counter, Some(("Value: 5".into(), false)));
        assert!(wi.true_text.is_none()); // counter-text is not simple-text
        assert!(wi.conjugations.is_none());
        assert_eq!(wi.score, Some(40));
    }

    #[tokio::test]
    async fn segment_with_no_score_passes_none_through() {
        // A segment with no score yields a word-info with no score (not 0).
        let ctx = ctx_from_env().await;
        let word = first_reading(&ctx, "ねこ").await;
        let mut seg = Segment {
            start: 0,
            end: 2,
            word,
            score: None,
            info: None,
            top: None,
            text: None,
        };
        let wi = word_info_from_segment(&ctx, &mut seg).await.unwrap();
        assert_eq!(wi.score, None);
    }

    #[tokio::test]
    async fn compound_text_segment_builds_components_with_primary_flag() {
        // A compound word builds one component word-info per child, with the
        // primary flag set only on the child whose seq matches the primary.
        use crate::dict::text_classes::{CompoundText, ScoreMod};
        let ctx = ctx_from_env().await;
        let w1 = first_reading(&ctx, "ねこ").await; // seq=1467640
        let w2 = first_reading(&ctx, "いぬ").await; // seq=1258330
        let compound = CompoundText {
            text: "ねこいぬ".into(),
            kana: "ねこいぬ".into(),
            primary: Box::new(w1.clone()),
            words: vec![w1, w2],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let mut seg = segment(KaniWordDispatchEnum::Compound(compound), 5, 0, 4);
        let wi = word_info_from_segment(&ctx, &mut seg).await.unwrap();
        assert_eq!(wi.text, "ねこいぬ");
        assert_eq!(
            wi.seq,
            Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(1467640)),
                Some(WordInfoSeq::Single(1258330)),
            ]))
        );
        assert_eq!(wi.components.len(), 2);
        assert!(wi.components[0].primary); // matches primary's seq
        assert!(!wi.components[1].primary); // different seq
    }
}

mod word_info_from_segment_list {
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::readings::{find_word, FindWordRows};
    use crate::dict::scoring::score::Segment;
    use crate::dict::word_info::WordInfoType;
    use crate::dict::word_info::*;
    // Needs a live Postgres DB.

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn one_kana_reading(ctx: &KaniranContext, word: &str) -> KaniWordDispatchEnum {
        let rows = find_word(ctx, word, false).await.unwrap().into_owned();
        match rows {
            FindWordRows::Kanji(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kanji)
                .expect("at least one kanji-text row"),
            FindWordRows::Kana(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kana)
                .expect("at least one kana-text row"),
        }
    }

    fn seg(word: KaniWordDispatchEnum, score: i32, start: usize, end: usize) -> Segment {
        Segment {
            start,
            end,
            word,
            score: Some(score),
            info: None,
            top: None,
            text: None,
        }
    }

    fn seg_list(segments: Vec<Segment>, start: usize, end: usize, matches: usize) -> SegmentList {
        SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches,
        }
    }

    #[tokio::test]
    async fn single_survivor_returns_wi1_with_skipped() {
        // One segment, matches=1 → single branch, skipped = matches - 1 = 0.
        let ctx = ctx_from_env().await;
        let word = one_kana_reading(&ctx, "ねこ").await;
        let mut sl = seg_list(vec![seg(word, 16, 0, 2)], 0, 2, 1);
        let wi = word_info_from_segment_list(&ctx, &mut sl).await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kana);
        assert_eq!(wi.text, "ねこ");
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(1467640)));
        assert!(!wi.alternative);
        assert_eq!(wi.skipped, 0);
        assert!(wi.components.is_empty());
        assert_eq!(wi.score, Some(16));
        assert_eq!(wi.start, Some(0));
        assert_eq!(wi.end, Some(2));
    }

    #[tokio::test]
    async fn single_survivor_skipped_eq_matches_minus_one() {
        // One survivor but matches=7 → skipped = matches - 1 = 6.
        let ctx = ctx_from_env().await;
        let word = one_kana_reading(&ctx, "ねこ").await;
        let mut sl = seg_list(vec![seg(word, 16, 0, 2)], 0, 2, 7);
        let wi = word_info_from_segment_list(&ctx, &mut sl).await.unwrap();
        assert_eq!(wi.skipped, 6);
    }

    #[tokio::test]
    async fn multi_survivor_builds_synthetic_from_wi1() {
        // Two surviving segments (both score 5, both pass the cutoff) build an
        // alternative word-info whose kind/text/score come from the first.
        let ctx = ctx_from_env().await;
        let neko = one_kana_reading(&ctx, "ねこ").await;
        let inu = one_kana_reading(&ctx, "いぬ").await;
        let mut sl = seg_list(vec![seg(neko, 5, 0, 2), seg(inu, 5, 0, 2)], 0, 2, 2);
        let wi = word_info_from_segment_list(&ctx, &mut sl).await.unwrap();
        assert!(wi.alternative);
        assert_eq!(wi.text, "ねこ"); // first segment's text
        assert_eq!(wi.score, Some(5));
        assert_eq!(wi.components.len(), 2);
        assert_eq!(wi.start, Some(0));
        assert_eq!(wi.end, Some(2));
        assert_eq!(wi.skipped, 0);
        assert_eq!(
            wi.seq,
            Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(1467640)),
                Some(WordInfoSeq::Single(1258330)),
            ]))
        );
    }

    #[tokio::test]
    async fn multi_branch_filters_below_two_thirds_of_wi1_score() {
        // wi1.score = 9, cutoff = (2*9)/3 = 6. Score-5 and score-3
        // segments fail; only wi1 survives → falls back to single branch.
        let ctx = ctx_from_env().await;
        let a = one_kana_reading(&ctx, "ねこ").await;
        let b = one_kana_reading(&ctx, "いぬ").await;
        let c = one_kana_reading(&ctx, "とり").await;
        let mut sl = seg_list(
            vec![seg(a, 9, 0, 1), seg(b, 5, 0, 1), seg(c, 3, 0, 1)],
            0,
            1,
            3,
        );
        let wi = word_info_from_segment_list(&ctx, &mut sl).await.unwrap();
        assert!(!wi.alternative);
        assert_eq!(wi.text, "ねこ");
        assert_eq!(wi.skipped, 2);
    }

    #[tokio::test]
    async fn multi_branch_anchors_kind_text_score_on_pre_filter_wi1() {
        // Constructed scenario: wi1 has the highest score; second
        // segment also passes the 2/3 cutoff. Confirms wi.text and
        // wi.score follow wi1 even when later survivors have different
        // scores.
        let ctx = ctx_from_env().await;
        let a = one_kana_reading(&ctx, "ねこ").await;
        let b = one_kana_reading(&ctx, "いぬ").await;
        let mut sl = seg_list(vec![seg(a, 9, 0, 1), seg(b, 7, 0, 1)], 0, 1, 2);
        let wi = word_info_from_segment_list(&ctx, &mut sl).await.unwrap();
        assert!(wi.alternative);
        assert_eq!(wi.text, "ねこ"); // wi1.text
        assert_eq!(wi.score, Some(9)); // wi1.score
    }

    #[test]
    fn dedup_keep_first_handles_heterogeneous_options() {
        // remove-duplicates :from-end t keeps first occurrence; the
        // collection is heterogeneous (Single, Multi, None).
        let a = Some(WordInfoKana::Single("a".into()));
        let b = Some(WordInfoKana::Single("b".into()));
        let nested = Some(WordInfoKana::Multi(vec![
            Some(WordInfoKana::Single("x".into())),
            Some(WordInfoKana::Single("y".into())),
        ]));
        let none: Option<WordInfoKana> = None;
        let result = dedup_keep_first(&[
            a.clone(),
            b.clone(),
            a.clone(),
            none.clone(),
            nested.clone(),
            none.clone(),
            nested.clone(),
        ]);
        assert_eq!(result, vec![a, b, none, nested]);
    }
}

mod word_info_from_text {
    use crate::dict::word_info::*;
    use crate::dict::word_info::{WordInfoKana, WordInfoSeq, WordInfoType};
    // Needs a live Postgres DB. Each case is a single-survivor result
    // (skipped = 0), so the outcome doesn't depend on row ordering; the
    // order-dependent multi-survivor cases live in
    // word_info_from_segment_list's tests.

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// "図書館" yields a single KANJI reading.
    #[tokio::test]
    async fn simple_kanji_noun() {
        let ctx = ctx_from_env().await;
        let wi = word_info_from_text(&ctx, "図書館").await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kanji);
        assert_eq!(wi.text, "図書館");
        assert_eq!(wi.kana, Some(WordInfoKana::Single("としょかん".into())));
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(1370420)));
        assert_eq!(wi.score, Some(952));
        assert!(!wi.alternative);
        assert_eq!(wi.skipped, 0);
        assert_eq!(wi.start, Some(0));
        assert_eq!(wi.end, Some(3));
        assert!(wi.components.is_empty());
        assert!(wi.counter.is_none());
        assert_eq!(wi.true_text.as_deref(), Some("図書館"));
        assert!(wi.conjugations.is_none());
        assert!(wi.primary);
    }

    /// "オレら" yields a single KANA reading. `end = 3` confirms the length
    /// is counted in characters, not bytes.
    #[tokio::test]
    async fn simple_kana_pronoun() {
        let ctx = ctx_from_env().await;
        let wi = word_info_from_text(&ctx, "オレら").await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kana);
        assert_eq!(wi.text, "オレら");
        assert_eq!(wi.kana, Some(WordInfoKana::Single("オレら".into())));
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(1576880)));
        assert_eq!(wi.score, Some(24));
        assert!(!wi.alternative);
        assert_eq!(wi.end, Some(3));
        assert!(wi.components.is_empty());
        assert_eq!(wi.true_text.as_deref(), Some("オレら"));
        assert!(wi.conjugations.is_none());
        assert!(wi.primary);
    }

    /// "食べてる" yields a single COMPOUND of 食べて + いる, with seq as the
    /// per-child list and one component per part.
    #[tokio::test]
    async fn compound_teiru() {
        let ctx = ctx_from_env().await;
        let wi = word_info_from_text(&ctx, "食べてる").await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kanji);
        assert_eq!(wi.text, "食べてる");
        assert_eq!(wi.kana, Some(WordInfoKana::Single("たべてる".into())));
        assert_eq!(
            wi.seq,
            Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(10092233)),
                Some(WordInfoSeq::Single(1577980)),
            ]))
        );
        assert_eq!(wi.score, Some(434));
        assert_eq!(wi.end, Some(4));
        assert_eq!(wi.components.len(), 2);
        assert_eq!(wi.components[0].text, "食べて");
        assert_eq!(wi.components[0].seq, Some(WordInfoSeq::Single(10092233)));
        assert_eq!(wi.components[1].text, "いる");
        assert_eq!(wi.components[1].seq, Some(WordInfoSeq::Single(1577980)));
        // A compound carries no true-text or conjugations.
        assert!(wi.true_text.is_none());
        assert!(wi.conjugations.is_none());
        assert!(wi.primary);
    }

    /// "5万100" resolves to a COUNTER reading: counter pair populated, no seq.
    /// `end = 5` is the character count (byte length is 7).
    #[tokio::test]
    async fn counter_auto_number() {
        let ctx = ctx_from_env().await;
        let wi = word_info_from_text(&ctx, "5万100").await.unwrap();
        assert_eq!(wi.text, "5万100");
        assert_eq!(wi.counter, Some(("Value: 50100".into(), false)));
        assert_eq!(wi.seq, None);
        assert_eq!(wi.score, Some(780));
        assert_eq!(wi.end, Some(5));
        assert!(wi.components.is_empty());
        // A counter reading carries no true-text or conjugations.
        assert!(wi.true_text.is_none());
        assert!(wi.conjugations.is_none());
        assert!(wi.primary);
    }

    /// "三羽" yields the counter reading (value 3).
    #[tokio::test]
    async fn counter_auto_kanji_number() {
        let ctx = ctx_from_env().await;
        let wi = word_info_from_text(&ctx, "三羽").await.unwrap();
        assert_eq!(wi.text, "三羽");
        assert_eq!(wi.kana, Some(WordInfoKana::Single("さんば".into())));
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(1607310)));
        assert_eq!(wi.counter, Some(("Value: 3".into(), false)));
        assert_eq!(wi.score, Some(286));
        assert_eq!(wi.end, Some(2));
        // A counter reading carries no true-text or conjugations.
        assert!(wi.true_text.is_none());
        assert!(wi.conjugations.is_none());
        assert!(wi.primary);
    }
}

mod fill_segment_path {
    use crate::dict::grammar::synergy::Synergy;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::readings::{find_word, FindWordRows};
    use crate::dict::scoring::score::Segment;
    use crate::dict::word_info::WordInfoSeq;
    use crate::dict::word_info::*;
    // Needs a live Postgres DB. Coverage:
    // - leading / internal / trailing gap insertion
    // - empty path with non-empty string emits one full-string gap
    // - empty path + empty string emits nothing
    // - synergy elements are filtered out
    // - char-indexed slicing (multibyte chars don't shift offsets)

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn first_reading(ctx: &KaniranContext, word: &str) -> KaniWordDispatchEnum {
        let rows = find_word(ctx, word, false).await.unwrap().into_owned();
        match rows {
            FindWordRows::Kanji(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kanji)
                .expect("no kanji rows"),
            FindWordRows::Kana(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kana)
                .expect("no kana rows"),
        }
    }

    async fn one_seg_list(
        ctx: &KaniranContext,
        word: &str,
        score: i32,
        start: usize,
        end: usize,
    ) -> SegmentList {
        let reading = first_reading(ctx, word).await;
        SegmentList {
            segments: vec![std::sync::Arc::new(Segment {
                start,
                end,
                word: reading,
                score: Some(score),
                info: None,
                top: None,
                text: None,
            })],
            start,
            end,
            top: None,
            matches: 1,
        }
    }

    #[tokio::test]
    async fn fills_internal_gap_between_two_segment_lists() {
        let ctx = ctx_from_env().await;
        let sl_neko = one_seg_list(&ctx, "ねこ", 16, 0, 2).await;
        let sl_inu = one_seg_list(&ctx, "いぬ", 16, 4, 6).await;
        let mut path = vec![
            PathElement::SegmentList(sl_neko),
            PathElement::SegmentList(sl_inu),
        ];
        let result = fill_segment_path(&ctx, "ねこと いぬ", &mut path)
            .await
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].text, "ねこ");
        assert_eq!(result[0].seq, Some(WordInfoSeq::Single(1467640)));
        assert_eq!(result[1].kind, WordInfoType::Gap);
        assert_eq!(result[1].text, "と ");
        assert_eq!(
            result[1].kana,
            Some(WordInfoKana::Single("と ".to_string()))
        );
        assert_eq!(result[1].start, Some(2));
        assert_eq!(result[1].end, Some(4));
        assert!(result[1].seq.is_none());
        assert_eq!(result[2].text, "いぬ");
        assert_eq!(result[2].seq, Some(WordInfoSeq::Single(1258330)));
    }

    #[tokio::test]
    async fn fills_leading_and_trailing_gap() {
        let ctx = ctx_from_env().await;
        let sl = one_seg_list(&ctx, "ねこ", 16, 2, 4).await;
        let mut path = vec![PathElement::SegmentList(sl)];
        let result = fill_segment_path(&ctx, "あいねこ犬", &mut path)
            .await
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].kind, WordInfoType::Gap);
        assert_eq!(result[0].text, "あい");
        assert_eq!(result[0].start, Some(0));
        assert_eq!(result[0].end, Some(2));
        assert_eq!(result[1].text, "ねこ");
        assert_eq!(result[2].kind, WordInfoType::Gap);
        assert_eq!(result[2].text, "犬");
        assert_eq!(result[2].start, Some(4));
        assert_eq!(result[2].end, Some(5));
    }

    #[tokio::test]
    async fn empty_path_with_text_emits_single_gap() {
        let ctx = ctx_from_env().await;
        let result = fill_segment_path(&ctx, "abcde", &mut []).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].kind, WordInfoType::Gap);
        assert_eq!(result[0].text, "abcde");
        assert_eq!(
            result[0].kana,
            Some(WordInfoKana::Single("abcde".to_string()))
        );
        assert_eq!(result[0].start, Some(0));
        assert_eq!(result[0].end, Some(5));
    }

    #[tokio::test]
    async fn empty_path_empty_string_emits_nothing() {
        let ctx = ctx_from_env().await;
        let result = fill_segment_path(&ctx, "", &mut []).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn segment_list_covers_entire_string_no_gap() {
        let ctx = ctx_from_env().await;
        let sl = one_seg_list(&ctx, "ねこ", 16, 0, 2).await;
        let mut path = vec![PathElement::SegmentList(sl)];
        let result = fill_segment_path(&ctx, "ねこ", &mut path).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "ねこ");
    }

    #[tokio::test]
    async fn synergy_elements_are_skipped() {
        let ctx = ctx_from_env().await;
        let sl_neko = one_seg_list(&ctx, "ねこ", 16, 0, 2).await;
        let sl_inu = one_seg_list(&ctx, "いぬ", 16, 4, 6).await;
        let mut path = vec![
            PathElement::SegmentList(sl_neko),
            PathElement::Synergy(Synergy {
                description: Some("stub".into()),
                connector: Some(" + ".into()),
                score: 5,
                start: 2,
                end: 4,
            }),
            PathElement::SegmentList(sl_inu),
        ];
        let result = fill_segment_path(&ctx, "ねこと いぬ", &mut path)
            .await
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].text, "ねこ");
        assert_eq!(result[1].kind, WordInfoType::Gap);
        assert_eq!(result[2].text, "いぬ");
    }
}

mod word_info_rec_find {
    use crate::dict::word_info::*;
    use crate::dict::word_info::{WordInfo, WordInfoType};
    // Walks a word-info tree (parent P with components こ / ねこ, sibling S)
    // and pairs each text-matching node with its successor's text.

    fn wi(text: &str, components: Vec<WordInfo>) -> WordInfo {
        WordInfo {
            kind: WordInfoType::Kana,
            text: text.to_string(),
            components,
            ..Default::default()
        }
    }

    fn tree() -> Vec<WordInfo> {
        let c1 = wi("こ", Vec::new());
        let c2 = wi("ねこ", Vec::new());
        let parent = wi("P", vec![c1, c2]);
        let sibling = wi("S", Vec::new());
        vec![parent, sibling]
    }

    fn pairs<'a>(result: &[(&'a WordInfo, Option<&'a WordInfo>)]) -> Vec<(String, Option<String>)> {
        result
            .iter()
            .map(|(car, cdr)| (car.text.clone(), cdr.map(|wi| wi.text.clone())))
            .collect()
    }

    #[test]
    fn rec_find_paths() {
        let tree = tree();
        let matches =
            |texts: &'static [&'static str]| move |wi: &WordInfo| texts.contains(&wi.text.as_str());

        // all-match: ((P . S) (こ . ねこ) (ねこ . S)) — parent emits before
        // its components; the last component's nil cdr falls back to wi-next S.
        assert_eq!(
            pairs(&word_info_rec_find(&tree, &matches(&["こ", "ねこ", "P"]))),
            vec![
                ("P".into(), Some("S".into())),
                ("こ".into(), Some("ねこ".into())),
                ("ねこ".into(), Some("S".into())),
            ]
        );

        // comp-only: parent fails the test; only the components match.
        assert_eq!(
            pairs(&word_info_rec_find(&tree, &matches(&["こ", "ねこ"]))),
            vec![
                ("こ".into(), Some("ねこ".into())),
                ("ねこ".into(), Some("S".into())),
            ]
        );

        // last-only: S is the final element → cdr is nil.
        assert_eq!(
            pairs(&word_info_rec_find(&tree, &matches(&["S"]))),
            vec![("S".into(), None)]
        );

        // parent-only: just the top-level match.
        assert_eq!(
            pairs(&word_info_rec_find(&tree, &matches(&["P"]))),
            vec![("P".into(), Some("S".into()))]
        );

        // no-match / empty list both yield nothing.
        assert!(word_info_rec_find(&tree, &|_: &WordInfo| false).is_empty());
        assert!(word_info_rec_find(&[], &|_: &WordInfo| true).is_empty());
    }
}

mod process_word_info {
    use crate::dict::word_info::WordInfoType;
    use crate::dict::word_info::*;

    fn wi(text: &str, kana: &str) -> WordInfo {
        WordInfo {
            kind: WordInfoType::Kanji,
            text: text.to_string(),
            kana: Some(WordInfoKana::Single(kana.to_string())),
            ..Default::default()
        }
    }

    #[test]
    fn nan_branch_voiced_t() {
        let list = process_word_info(vec![wi("何", "なに"), wi("で", "で")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なん".to_string())));
    }

    #[test]
    fn nani_branch_unvoiced_k() {
        let list = process_word_info(vec![wi("何", "なん"), wi("か", "か")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なに".to_string())));
    }

    #[test]
    fn nani_branch_vowel() {
        let list = process_word_info(vec![wi("何", "なん"), wi("ある", "ある")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なに".to_string())));
    }

    #[test]
    fn ni_treated_as_nani() {
        let list = process_word_info(vec![wi("何", "なん"), wi("人", "にん")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なに".to_string())));
    }

    #[test]
    fn no_next_word_unchanged() {
        let list = process_word_info(vec![wi("何", "なん")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なん".to_string())));
    }

    #[test]
    fn non_target_text_unchanged() {
        let list = process_word_info(vec![wi("猫", "ねこ"), wi("で", "で")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("ねこ".to_string())));
    }

    #[test]
    fn multi_kana_mixed_picks_nani() {
        let mut next_wi = wi("X", "");
        next_wi.kana = Some(WordInfoKana::Multi(vec![
            Some(WordInfoKana::Single("で".to_string())),
            Some(WordInfoKana::Single("か".to_string())),
        ]));
        let list = process_word_info(vec![wi("何", "なん"), next_wi]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なに".to_string())));
    }

    #[test]
    fn empty_kana_no_change() {
        let mut next_wi = wi("X", "");
        next_wi.kana = Some(WordInfoKana::Multi(Vec::new()));
        let list = process_word_info(vec![wi("何", "なん"), next_wi]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なん".to_string())));
    }
}

mod word_info_reading {
    use crate::dict::word_info::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn wi(kind: WordInfoType, true_text: Option<&str>) -> WordInfo {
        WordInfo {
            kind,
            true_text: true_text.map(str::to_owned),
            ..Default::default()
        }
    }

    /// Looks up the dictionary row for a word-info's true-text. Covers the
    /// KANJI type (学校, 図書館), the KANA type (ねこ, きそうてんがい), a GAP type
    /// (no lookup → None), missing true-text (→ None), and a true-text with
    /// no matching row (→ None).
    #[tokio::test]
    async fn word_info_reading_fixtures() {
        let ctx = ctx_from_env().await;

        let cases: &[(WordInfo, Option<(i32, i32, bool)>)] = &[
            // (word-info, Some((seq, id, is_kanji)) | None)
            (
                wi(WordInfoType::Kanji, Some("学校")),
                Some((1206730, 9064, true)),
            ),
            (
                wi(WordInfoType::Kanji, Some("図書館")),
                Some((1370420, 29808, true)),
            ),
            (
                wi(WordInfoType::Kana, Some("ねこ")),
                Some((1467640, 54168, false)),
            ),
            (
                wi(WordInfoType::Kana, Some("きそうてんがい")),
                Some((1219430, 28651, false)),
            ),
            (wi(WordInfoType::Gap, Some("学校")), None),
            (wi(WordInfoType::Kanji, None), None),
            (
                wi(WordInfoType::Kanji, Some("存在しない漢字列 abcxyz")),
                None,
            ),
        ];

        for (word_info, expected) in cases {
            let result = word_info_reading(&ctx, word_info).await.unwrap();
            match (expected, result) {
                (None, None) => {}
                (Some((seq, id, is_kanji)), Some(KaniWordDispatchEnum::Kanji(row))) => {
                    assert!(
                        *is_kanji,
                        "true_text={:?}: expected kana-text",
                        word_info.true_text
                    );
                    assert_eq!(row.seq, *seq, "true_text={:?}", word_info.true_text);
                    assert_eq!(row.id, *id, "true_text={:?}", word_info.true_text);
                    assert_eq!(&row.text, word_info.true_text.as_ref().unwrap());
                }
                (Some((seq, id, is_kanji)), Some(KaniWordDispatchEnum::Kana(row))) => {
                    assert!(
                        !*is_kanji,
                        "true_text={:?}: expected kanji-text",
                        word_info.true_text
                    );
                    assert_eq!(row.seq, *seq, "true_text={:?}", word_info.true_text);
                    assert_eq!(row.id, *id, "true_text={:?}", word_info.true_text);
                    assert_eq!(&row.text, word_info.true_text.as_ref().unwrap());
                }
                (expected, result) => panic!(
                    "true_text={:?}: expected {expected:?}, got variant mismatch ({})",
                    word_info.true_text,
                    result.is_some()
                ),
            }
        }
    }
}

mod dict_segment {
    use crate::dict::word_info::WordInfoType;
    use crate::dict::word_info::*;
    // Needs a live Postgres DB. Coverage:
    // - multi-path result, scores descending
    // - limit caps the number of paths
    // - default limit (None) resolves to 5
    // - empty string yields one seed path with an empty word-info-list
    // - all-gap input yields one path with the gap-penalty score

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn texts(word_info_list: &[WordInfo]) -> Vec<String> {
        word_info_list
            .iter()
            .map(|wi| {
                if wi.kind == WordInfoType::Gap {
                    ":GAP".to_string()
                } else {
                    wi.text.clone()
                }
            })
            .collect()
    }

    #[tokio::test]
    async fn multi_path_scores_descending() {
        let ctx = ctx_from_env().await;
        let result = dict_segment(&ctx, "しませんか", Some(3)).await.unwrap();
        let scores: Vec<i32> = result.iter().map(|(_, score)| *score).collect();
        assert_eq!(scores, vec![352, 52, 48]);
        assert_eq!(texts(&result[0].0), vec!["しません", "か"]);
        assert_eq!(texts(&result[1].0), vec!["しま", "せんか"]);
        assert_eq!(texts(&result[2].0), vec!["しま", "せん", "か"]);
    }

    #[tokio::test]
    async fn limit_one_returns_single_best_path() {
        let ctx = ctx_from_env().await;
        let result = dict_segment(&ctx, "しませんか", Some(1)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 352);
        assert_eq!(texts(&result[0].0), vec!["しません", "か"]);
    }

    #[tokio::test]
    async fn default_limit_is_five() {
        let ctx = ctx_from_env().await;
        let result = dict_segment(&ctx, "ご注文はうさぎですか", None)
            .await
            .unwrap();
        let scores: Vec<i32> = result.iter().map(|(_, score)| *score).collect();
        assert_eq!(scores, vec![518, 504, 485, 474, 465]);
        assert_eq!(
            texts(&result[0].0),
            vec!["ご注文", "は", "うさぎ", "です", "か"]
        );
    }

    #[tokio::test]
    async fn empty_string_seeds_one_empty_path() {
        let ctx = ctx_from_env().await;
        let result = dict_segment(&ctx, "", Some(5)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 0);
        assert!(result[0].0.is_empty());
    }

    #[tokio::test]
    async fn all_gap_input_one_path_with_gap_penalty() {
        let ctx = ctx_from_env().await;
        let result = dict_segment(&ctx, "abcde", Some(5)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, -2500);
        assert_eq!(texts(&result[0].0), vec![":GAP"]);
    }
}

mod simple_segment {
    use crate::dict::word_info::WordInfoType;
    use crate::dict::word_info::*;
    // The segmentation test: each input string maps to the expected sequence
    // of segments (word text, or GAP for an unsegmented run). Needs a live
    // Postgres DB.

    const GAP: &str = ":GAP";

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    // Gap word-infos map to GAP, others to their text.
    fn segmentation(word_info_list: &[WordInfo]) -> Vec<&str> {
        word_info_list
            .iter()
            .map(|wi| {
                if wi.kind == WordInfoType::Gap {
                    GAP
                } else {
                    wi.text.as_str()
                }
            })
            .collect()
    }

    #[tokio::test]
    async fn segmentation_test() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, &[&str])] = &[
            (
                "ご注文はうさぎですか",
                &["ご注文", "は", "うさぎ", "です", "か"],
            ),
            ("しませんか", &["しません", "か"]),
            ("ドンマイ", &["ドンマイ"]),
            ("みんな土足でおいで", &["みんな", "土足で", "おいで"]),
            ("おもわぬオチ提供中", &["おもわぬ", "オチ", "提供", "中"]),
            ("わたし", &["わたし"]),
            (
                "お姉ちゃんにまかせて地球まるごと",
                &["お姉ちゃん", "に", "まかせて", "地球", "まるごと"],
            ),
            ("名人になってるはず", &["名人", "に", "なってる", "はず"]),
            ("いいとこ", &["いいとこ"]),
            ("そういうお隣どうし", &["そういう", "お", "隣どうし"]),
            ("はしゃいじゃう", &["はしゃいじゃう"]),
            ("分かっちゃうのよ", &["分かっちゃう", "の", "よ"]),
            (
                "懐かしく新しいまだそしてまた",
                &["懐かしく", "新しい", "まだ", "そして", "また"],
            ),
            (
                "あたりまえみたいに思い出いっぱい",
                &["あたりまえ", "みたい", "に", "思い出", "いっぱい"],
            ),
            (
                "何でもない日々とっておきのメモリアル",
                &["何でもない", "日々", "とっておき", "の", "メモリアル"],
            ),
            (
                "しつれいしなければならないんです",
                &["しつれいし", "なければならない", "ん", "です"],
            ),
            (
                "だけど気付けば馴染んじゃってる",
                &["だけど", "気付けば", "馴染んじゃってる"],
            ),
            ("飲んで笑っちゃえば", &["飲んで", "笑っちゃえば"]),
            ("なんで", &["なんで"]),
            ("遠慮しないでね", &["遠慮しないで", "ね"]),
            ("出かけるまえに", &["出かける", "まえ", "に"]),
            ("感じたいでしょ", &["感じたい", "でしょ"]),
            ("まじで", &["まじ", "で"]),
            (
                "その山を越えたとき",
                &["その", "山", "を", "越えた", "とき"],
            ),
            ("遊びたいのに", &["遊びたい", "のに"]),
            ("しながき", &["しながき"]),
            ("楽しさ求めて", &["楽しさ", "求めて"]),
            ("日常のなかにも", &["日常", "の", "なかにも"]),
            (
                "ほんとは好きなんだと",
                &["ほんと", "は", "好き", "な", "ん", "だ", "と"],
            ),
            ("内緒なの", &["内緒", "なの"]),
            ("魚が好きじゃない", &["魚", "が", "好き", "じゃない"]),
            ("物語になってく", &["物語", "に", "なってく"]),
            ("書いてきてくださった", &["書いてきて", "くださった"]),
            ("今日は何の日", &["今日", "は", "何の", "日"]),
            ("何から話そうか", &["何", "から", "話そう", "か"]),
            ("話したくなる", &["話したくなる"]),
            ("進化してく友情", &["進化してく", "友情"]),
            ("私に任せてくれ", &["私", "に", "任せてくれ"]),
            (
                "時までに帰ってくると約束してくれるのなら外出してよろしい",
                &[
                    "時",
                    "まで",
                    "に",
                    "帰ってくる",
                    "と",
                    "約束してくれる",
                    "の",
                    "なら",
                    "外出して",
                    "よろしい",
                ],
            ),
            (
                "雪が降りそうな気がします",
                &["雪", "が", "降りそう", "な", "気がします"],
            ),
            ("新しそうだ", &["新しそう", "だ"]),
            (
                "本を読んだりテレビを見たりします",
                &["本", "を", "読んだり", "テレビ", "を", "見たり", "します"],
            ),
            (
                "今日母はたぶんうちにいるでしょう",
                &[
                    "今日",
                    "母",
                    "は",
                    "たぶん",
                    "うち",
                    "に",
                    "いる",
                    "でしょう",
                ],
            ),
            ("赤かったろうです", &["赤かったろう", "です"]),
            ("そう呼んでくれていい", &["そう", "呼んでくれていい"]),
            ("払わなくてもいい", &["払わなくてもいい"]),
            (
                "体に悪いと知りながらタバコをやめることはできない",
                &[
                    "体に悪い",
                    "と",
                    "知り",
                    "ながら",
                    "タバコをやめる",
                    "こと",
                    "は",
                    "できない",
                ],
            ),
            ("微笑みはまぶしすぎる", &["微笑み", "は", "まぶしすぎる"]),
            ("なにをしていますか", &["なに", "を", "しています", "か"]),
            (
                "優しすぎそのうえカッコいいの",
                &["優しすぎ", "そのうえ", "カッコいい", "の"],
            ),
            (
                "この本は複雑すぎるから",
                &["この", "本", "は", "複雑", "すぎる", "から"],
            ),
            ("かわいいです", &["かわいいです"]),
            ("学生なんだ", &["学生", "な", "ん", "だ"]),
            ("なんだから", &["な", "ん", "だから"]),
            ("名付けたい", &["名付けたい"]),
            ("切なくなってしまう", &["切なくなってしまう"]),
            ("らいかな", &["らい", "かな"]),
            ("誰かいなくなった", &["誰か", "いなくなった"]),
            ("思い出すな", &["思い出す", "な"]),
            ("かなって思ったら", &["かなって", "思ったら"]),
            (
                "法律にかなっているさま",
                &["法律", "に", "かなっている", "さま"],
            ),
            ("ことすら難しい", &["こと", "すら", "難しい"]),
            ("投下しました", &["投下しました"]),
            ("車止める", &["車", "止める"]),
            ("円盤はただの", &["円盤", "は", "ただ", "の"]),
            (
                "ズボンからすねをむき出しにする",
                &["ズボン", "から", "すね", "を", "むき", "出しにする"],
            ),
            (
                "駅の前で会いましょう",
                &["駅", "の", "前", "で", "会いましょう"],
            ),
            (
                "あなたの質問は答えにくい",
                &["あなた", "の", "質問", "は", "答えにくい"],
            ),
            ("とかそういう", &["とか", "そういう"]),
            ("好評のうちに", &["好評", "の", "うち", "に"]),
            (
                "映像もすごくよかったです",
                &["映像", "も", "すごく", "よかったです"],
            ),
            ("情けねえ", &["情けねえ"]),
            ("春ですねえ", &["春", "です", "ねえ"]),
            ("春ですねぇ", &["春", "です", "ねぇ"]),
            ("きつねじゃなかった", &["きつね", "じゃなかった"]),
            (
                "ワシじゃなくて和紙じゃよ",
                &["ワシ", "じゃなくて", "和紙", "じゃ", "よ"],
            ),
            ("ほうがいいよ", &["ほうがいい", "よ"]),
            (
                "痛さはどれくらいですか",
                &["痛さ", "は", "どれくらい", "です", "か"],
            ),
            ("見てくれたかな", &["見てくれた", "かな"]),
            ("とても良かった", &["とても", "良かった"]),
            (
                "戻りたいかと言われる",
                &["戻りたい", "か", "と", "言われる"],
            ),
            (
                "こういうのでいいんだよ",
                &["こういう", "の", "でいい", "ん", "だ", "よ"],
            ),
            (
                "そんなのでいいと思ってるの",
                &["そんな", "の", "でいい", "と", "思ってる", "の"],
            ),
            ("だけが墓参りしてた", &["だけ", "が", "墓参りしてた"]),
            ("はいいんだけどな", &["は", "いい", "ん", "だけど", "な"]),
            ("なりつつあるんだが", &["なりつつある", "ん", "だが"]),
            ("反論は認めません", &["反論", "は", "認めません"]),
            ("見たような気がする", &["見た", "ような気がする"]),
            (
                "幽霊を見たような顔つきをしていた",
                &["幽霊", "を", "見た", "ような", "顔つき", "を", "していた"],
            ),
            ("元気になる", &["元気", "に", "なる"]),
            ("半端なかった", &["半端なかった"]),
            ("一人ですね", &["一人", "です", "ね"]),
            ("行事がある", &["行事", "が", "ある"]),
            ("当てられたものになる", &["当てられた", "ものになる"]),
            ("獲得しうる", &["獲得しうる"]),
            ("ことができず", &["ことができず"]),
            (
                "一生一度だけの忘られぬ約束",
                &["一生一度", "だけ", "の", "忘られぬ", "約束"],
            ),
            (
                "やらずにこの路線でよかったのに",
                &["やらず", "に", "この", "路線", "で", "よかった", "のに"],
            ),
            ("歌ってしまいそう", &["歌ってしまいそう"]),
            ("しまいそう", &["しまいそう"]),
            ("まいそう祭り", &["まいそう", "祭り"]),
            ("何ですか", &["何", "です", "か"]),
            ("浮かれたいから", &["浮かれたい", "から"]),
            ("なくなっちゃう", &["なくなっちゃう"]),
            ("になりそうだけど", &["に", "なりそう", "だけど"]),
            (
                "これは辛い選択になりそうだな",
                &["これ", "は", "辛い", "選択", "に", "なりそう", "だ", "な"],
            ),
            ("はっきりしそうだな", &["はっきりしそう", "だ", "な"]),
            ("泣きそうなんだけど", &["泣きそう", "な", "ん", "だけど"]),
            ("これですね", &["これ", "です", "ね"]),
            ("はいなくなります", &["は", "いなくなります"]),
            ("忘れなく", &["忘れなく"]),
            ("じゃないですか", &["じゃないです", "か"]),
            ("純粋さ健気さ", &["純粋さ", "健気さ"]),
            ("着てたからね", &["着てた", "から", "ね"]),
            (
                "仕出かすからだと思います",
                &["仕出かす", "から", "だ", "と", "思います"],
            ),
            ("みんながした", &["みんな", "が", "した"]),
            ("ほうが速いと", &["ほう", "が", "速い", "と"]),
            ("注意してください", &["注意してください"]),
            (
                "昨日といいどうしてこう",
                &["昨日", "と", "いい", "どうして", "こう"],
            ),
            ("いっぱいきそう", &["いっぱい", "きそう"]),
            ("仲良しになったら", &["仲良し", "に", "なったら"]),
            ("全くといっていい", &["全く", "と", "いって", "いい"]),
            ("発狂しそうなんだ", &["発狂しそう", "な", "ん", "だ"]),
            ("していたんだ", &["していた", "ん", "だ"]),
            ("引き上げられた", &["引き上げられた"]),
            ("をつかむため", &["を", "つかむ", "ため"]),
            ("ときが自分", &["とき", "が", "自分"]),
            ("もうこころ", &["もう", "こころ"]),
            ("届けしたら", &["届け", "したら"]),
            (
                "おまえら低いんだよ",
                &["おまえら", "低い", "ん", "だ", "よ"],
            ),
            (
                "すべてがかかっていると思いながら",
                &["すべて", "が", "かかっている", "と", "思い", "ながら"],
            ),
            ("エロいと思っちゃう", &["エロい", "と", "思っちゃう"]),
            ("変わり映えしない", &["変わり映えしない"]),
            (
                "あなたがいなきゃこんな計画思いつかなかった",
                &[
                    "あなた",
                    "が",
                    "いなきゃ",
                    "こんな",
                    "計画",
                    "思いつかなかった",
                ],
            ),
            ("見たかったです", &["見たかったです"]),
            ("出来て楽しかったな", &["出来て", "楽しかった", "な"]),
            ("つかってください", &["つかってください"]),
            ("誰もが思ってた", &["誰も", "が", "思ってた"]),
            ("参考にしたらしい", &["参考にしたらしい"]),
            ("狙いやすそうで", &["狙い", "やすそう", "で"]),
            (
                "予定はございませんので",
                &["予定", "は", "ございません", "ので"],
            ),
            (
                "犬はトラックにはねられた",
                &["犬", "は", "トラック", "に", "はねられた"],
            ),
            ("仕事してください", &["仕事してください"]),
            ("おいかけっこしましょ", &["おい", "かけっこしましょ"]),
            (
                "イラストカードが付きます",
                &["イラスト", "カード", "が", "付きます"],
            ),
            ("じゃないかしら", &["じゃない", "かしら"]),
            ("いつか本当に", &["いつか", "本当に"]),
            ("言い方もします", &["言い方", "も", "します"]),
            ("何でこれ", &["何で", "これ"]),
            (
                "こういう物語ができるんだ",
                &["こういう", "物語", "が", "できる", "ん", "だ"],
            ),
            (
                "といったところでしょうか",
                &["といった", "ところ", "でしょうか"],
            ),
            ("広めたいと思っている", &["広めたい", "と", "思っている"]),
            ("のせいかな", &["の", "せい", "かな"]),
            ("その場合", &["その", "場合"]),
            ("教えてくれてありがとう", &["教えてくれて", "ありがとう"]),
            (
                "彼が来るかどうか疑問だ",
                &["彼", "が", "来る", "かどうか", "疑問", "だ"],
            ),
            (
                "泳ぎに行ってはどうかな",
                &["泳ぎ", "に", "行って", "は", "どうかな"],
            ),
            (
                "どうか僕を許して下さい",
                &["どうか", "僕", "を", "許して", "下さい"],
            ),
            ("鏡はいらないですよ", &["鏡", "は", "いらないです", "よ"]),
            (
                "ベッドで跳ねちゃいけません",
                &["ベッド", "で", "跳ねちゃ", "いけません"],
            ),
            (
                "お酒を飲んじゃだめです",
                &["お酒", "を", "飲んじゃ", "だめ", "です"],
            ),
            ("これ洗濯しといて", &["これ", "洗濯しといて"]),
            (
                "来週までに読んどいて",
                &["来週", "まで", "に", "読んどいて"],
            ),
            (
                "奴がまともに見られない",
                &["奴", "が", "まともに", "見られない"],
            ),
            ("間違いなし", &["間違いなし"]),
            ("見ませんでしょうか", &["見ません", "でしょうか"]),
            (
                "書いていただけませんでしょうか",
                &["書いていただけません", "でしょうか"],
            ),
            ("友達できる", &["友達", "できる"]),
            ("実はそうなんだ", &["実は", "そう", "なんだ"]),
            ("やらしいです", &["やらしいです"]),
            ("荒いとこもある", &["荒い", "とこ", "も", "ある"]),
            ("あったかいとこ行こう", &["あったかい", "とこ", "行こう"]),
            ("ぶっちゃけ話", &["ぶっちゃけ", "話"]),
            ("いけないわー", &["いけない", "わ", GAP]),
            (
                "社長としてやっていけないわ",
                &["社長", "として", "やっていけない", "わ"],
            ),
            ("よくわかんないけど", &["よく", "わかんない", "けど"]),
            (
                "ほうがいいんじゃないの",
                &["ほうがいい", "ん", "じゃない", "の"],
            ),
            ("こんなんじゃ", &["こんなん", "じゃ"]),
            ("増やしたほうがいいな", &["増やした", "ほうがいい", "な"]),
            ("屈しやすいものだ", &["屈し", "やすい", "もの", "だ"]),
            ("目をもっている", &["目", "を", "もっている"]),
            (
                "これが君のなすべきものだ",
                &["これ", "が", "君", "の", "なすべき", "もの", "だ"],
            ),
            ("泥棒をつかまえた", &["泥棒", "を", "つかまえた"]),
            (
                "金もないし友達もいません",
                &["金", "も", "ない", "し", "友達", "も", "いません"],
            ),
            (
                "出来たからほら見てよ",
                &["出来た", "から", "ほら", "見て", "よ"],
            ),
            (
                "眠いからもう寝るね",
                &["眠い", "から", "もう", "寝る", "ね"],
            ),
            ("浮気してやがった", &["浮気してやがった"]),
            ("見本通りに", &["見本", "通り", "に"]),
            ("不適応", &["不", "適応"]),
            ("良いそうです", &["良い", "そう", "です"]),
            ("むらむらとわいた", &["むらむら", "と", "わいた"]),
            ("否定しちゃいけない", &["否定しちゃ", "いけない"]),
            ("観たいです", &["観たいです"]),
            ("あんたはわからん", &["あんた", "は", "わからん"]),
            ("見られたくないとこ", &["見られたくない", "とこ"]),
            ("多分家で", &["多分", "家", "で"]),
            ("三十八", &["三十八"]),
            (
                "エロそうだヤバそうだ",
                &["エロそう", "だ", "ヤバそう", "だ"],
            ),
            ("私にとっても", &["私", "にとって", "も"]),
            (
                "睡眠を十分にとってください",
                &["睡眠", "を", "十分", "に", "とってください"],
            ),
            ("そうなんだけど", &["そう", "な", "ん", "だけど"]),
            ("進んでない", &["進んでない"]),
            (
                "一回だけであとは言わない",
                &["一回", "だけ", "で", "あと", "は", "言わない"],
            ),
            (
                "ご親切に恐縮しております",
                &["ご親切に", "恐縮しております"],
            ),
            (
                "官吏となっておる者がある",
                &["官吏", "と", "なっておる", "者", "が", "ある"],
            ),
            (
                "間違えておられたようですね",
                &["間違えておられた", "ようです", "ね"],
            ),
            ("人気のせいな", &["人気", "の", "せい", "な"]),
            ("コレはアレ", &["コレ", "は", "アレ"]),
            ("アレハレ", &["アレ", GAP]),
            (
                "上に文字があったり",
                &["上", "に", "文字", "が", "あったり"],
            ),
            ("言っただろ", &["言った", "だろ"]),
            (
                "嵐が起ころうとしている",
                &["嵐", "が", "起ころうとしている"],
            ),
            ("知らないでしょう", &["知らないでしょう"]),
            ("読まないでしょう", &["読まないでしょう"]),
            ("来ないでしょう", &["来ないでしょう"]),
            ("何もかもがめんどい", &["何もかも", "が", "めんどい"]),
            ("なにもかもがめんどい", &["なにもかも", "が", "めんどい"]),
            (
                "あいつ規制されりゃいいのに",
                &["あいつ", "規制されりゃ", "いい", "のに"],
            ),
            (
                "塗ってみようと思って",
                &["塗って", "みよう", "と", "思って"],
            ),
            ("肩を並べられなかった", &["肩を並べられなかった"]),
            ("じゃなくて良かった", &["じゃなくて", "良かった"]),
            ("申し訳なさそう", &["申し訳なさそう"]),
            ("決まってたし", &["決まってた", "し"]),
            ("決まっている", &["決まっている"]),
            ("恐れ入りました", &["恐れ入りました"]),
            ("はうまい", &["は", "うまい"]),
            ("弾け飛びました", &["弾け飛びました"]),
            ("ぶっこんでいるようで", &["ぶっこんでいる", "よう", "で"]),
            ("じゃないけど下手に", &["じゃない", "けど", "下手", "に"]),
            ("的にそうではない", &["的", "に", "そう", "ではない"]),
            ("入り込めなかった", &["入り込めなかった"]),
            ("がいまいちなんだよ", &["が", "いまいち", "なんだ", "よ"]),
            ("脱がしにかかってる", &["脱がし", "に", "かかってる"]),
            ("必死になってる", &["必死", "に", "なってる"]),
            ("安心させた", &["安心させた"]),
            ("人が好きそうだ", &["人", "が", "好き", "そう", "だ"]),
            ("もっていこうとする", &["もっていこうとする"]),
            ("増やして", &["増やして"]),
            ("ぜいたくで", &["ぜいたく", "で"]),
            ("したくらいで", &["したくらい", "で"]),
            ("でもうまく人", &["でも", "うまく", "人"]),
            (
                "好き嫌いもしないように",
                &["好き嫌い", "も", "しない", "ように"],
            ),
            ("のどこが思える", &["の", "どこ", "が", "思える"]),
            ("出会えて良かった", &["出会えて", "良かった"]),
            ("無理しなくていいから", &["無理しなくていい", "から"]),
            ("調子にのらないほうが", &["調子にのらない", "ほう", "が"]),
            ("こなさそう", &["こなさそう"]),
            ("伸びてこなさそう", &["伸びてこなさそう"]),
            ("手にとって", &["手にとって"]),
            ("平和である", &["平和", "で", "ある"]),
            (
                "私にとっては少しおかしいです",
                &["私", "にとって", "は", "少し", "おかしいです"],
            ),
            ("パーティーは", &["パーティー", "は"]),
            (
                "彼以上のばかはいない",
                &["彼", "以上", "の", "ばか", "は", "いない"],
            ),
            (
                "君がいないと淋しい",
                &["君", "が", "いない", "と", "淋しい"],
            ),
            ("思いきって", &["思いきって"]),
            ("思いきっている", &["思いきっている"]),
            ("大事になります", &["大事", "に", "なります"]),
            ("元気にします", &["元気", "に", "します"]),
            (
                "ご迷惑おかけしてすみません",
                &["ご迷惑", "おかけして", "すみません"],
            ),
            (
                "不便をおかけすることを謝ります",
                &["不便", "を", "おかけする", "こと", "を", "謝ります"],
            ),
            (
                "お手数おかけし申し訳ないが",
                &["お手数", "おかけし", "申し訳ない", "が"],
            ),
            (
                "私はあなたにお手数をおかけました",
                &[
                    "私",
                    "は",
                    "あなた",
                    "に",
                    "お手数",
                    "を",
                    "お",
                    "かけました",
                ],
            ),
            ("ここにおかけなさい", &["ここ", "に", "お", "かけなさい"]),
            ("弾き出されてる", &["弾き出されてる"]),
            ("あかんわ", &["あかん", "わ"]),
            ("ぶっちゃけ", &["ぶっちゃけ"]),
            ("賢人たち", &["賢人", "たち"]),
            ("差ついた", &["差", "ついた"]),
            ("ですら", &["ですら"]),
            ("でさえ", &["でさえ"]),
            ("みごとにやってのける", &["みごと", "に", "やってのける"]),
            ("いる", &["いる"]),
            ("はいずれ", &["は", "いずれ"]),
            ("お下がり", &["お下がり"]),
            (
                "でも1000台とか1桁はあんまりだよな",
                &[
                    "でも",
                    "1000台",
                    "とか",
                    "1桁",
                    "は",
                    "あんまり",
                    "だ",
                    "よな",
                ],
            ),
            (
                "みんなにうらやましがられている",
                &["みんな", "に", "うらやましがられている"],
            ),
            ("悪がられて", &["悪がられて"]),
            (
                "期待されがちなので男女",
                &["期待されがち", "なので", "男女"],
            ),
            ("とぎれがちに話す", &["とぎれがち", "に", "話す"]),
            (
                "手にとっていただきやすくなる",
                &["手にとって", "いただき", "やすくなる"],
            ),
            ("さほど", &["さほど"]),
            ("大きさほどもある", &["大きさ", "ほど", "も", "ある"]),
            ("しかいない", &["しか", "いない"]),
            ("掴めていない", &["掴めていない"]),
            ("振り回されたいな", &["振り回されたい", "な"]),
            ("さぼっている", &["さぼっている"]),
            ("のままで来る", &["の", "まま", "で", "来る"]),
            ("5人中4人", &["5人中", "4人"]),
            (
                "彼はどなりすぎて声をからした",
                &["彼", "は", "どなり", "すぎて", "声", "を", "からした"],
            ),
            (
                "そうしたいからしただけだ",
                &["そう", "したい", "から", "した", "だけ", "だ"],
            ),
            ("推し続けている", &["推し", "続けている"]),
            ("少し直せたら", &["少し", "直せたら"]),
            ("良いほう", &["良い", "ほう"]),
            ("いいえ", &["いいえ"]),
            ("割り当てられた", &["割り当てられた"]),
            (
                "綺麗だけど近よりがたいよね",
                &["綺麗", "だけど", "近よりがたい", "よね"],
            ),
            ("そうなんじゃない", &["そう", "な", "ん", "じゃない"]),
            ("なんというかすみません", &["なんというか", "すみません"]),
            ("めんどくそがる", &["めんどくそがる"]),
            ("がなんで終わった", &["が", "なんで", "終わった"]),
            (
                "てか最近ファン層は円盤すら買わないからそいつらから金とるってのは無謀",
                &[
                    "てか",
                    "最近",
                    "ファン層",
                    "は",
                    "円盤",
                    "すら",
                    "買わない",
                    "から",
                    "そいつら",
                    "から",
                    "金",
                    "とる",
                    "ってのは",
                    "無謀",
                ],
            ),
            ("とろいな", &["とろい", "な"]),
            ("なんでもかんでも", &["なんでもかんでも"]),
            ("しないかい", &["しない", "かい"]),
            (
                "参拝しちゃいかんという人がいます",
                &["参拝しちゃ", "いかん", "という", "人", "が", "います"],
            ),
            (
                "人をひやかしちゃいやよ",
                &["人", "を", "ひやかしちゃ", "いや", "よ"],
            ),
            ("しちゃいたい", &["しちゃいたい"]),
            (
                "けがなどをしないように",
                &["けが", "など", "を", "しない", "ように"],
            ),
            ("買い支えたいと思う", &["買い", "支えたい", "と", "思う"]),
            ("おじゃましています", &["おじゃましています"]),
            ("とかいらんから", &["とか", "いらん", "から"]),
            (
                "ということだろうけど",
                &["という", "こと", "だろう", "けど"],
            ),
            (
                "のはわからなくもない",
                &["の", "は", "わからなく", "も", "ない"],
            ),
            ("変わっていくだろう", &["変わっていく", "だろう"]),
            ("待ってねぇ", &["待って", "ねぇ"]),
            (
                "おかしいと思わんですか",
                &["おかしい", "と", "思わん", "です", "か"],
            ),
            ("ズレてる", &["ズレてる"]),
            ("紅茶飲みたい", &["紅茶", "飲みたい"]),
            ("電気がついた", &["電気", "が", "ついた"]),
            ("脚本会議", &["脚本", "会議"]),
            (
                "見せなきゃいけなくなって",
                &["見せなきゃ", "いけなくなって"],
            ),
            (
                "私じゃなくなるような瞬間があって",
                &["私", "じゃなくなる", "ような", "瞬間", "が", "あって"],
            ),
            ("効いててかなりぬくい", &["効いてて", "かなり", "ぬくい"]),
            ("撮影してていつもは", &["撮影してて", "いつも", "は"]),
            (
                "むしろいないほうが珍しい",
                &["むしろ", "いない", "ほう", "が", "珍しい"],
            ),
            ("旅行にいきたい", &["旅行", "に", "いきたい"]),
            (
                "見ててこんな話あったっけ",
                &["見てて", "こんな", "話", "あった", "っけ"],
            ),
            ("いじめとかある", &["いじめ", "とか", "ある"]),
            ("となったらしい", &["となったらしい"]),
            ("基地外が必死過ぎ", &["基地外", "が", "必死", "過ぎ"]),
            ("調整のせいとか", &["調整", "の", "せい", "とか"]),
            ("はっしていない", &["はっしていない"]),
            ("無理さえしなければ", &["無理", "さえ", "しなければ"]),
            ("ところで", &["ところで"]),
            ("外に出て", &["外", "に", "出て"]),
            ("大人しそうな顔", &["大人しそう", "な", "顔"]),
            (
                "おとなしそうなようすにだまされた",
                &["おとなしそう", "な", "ようす", "に", "だまされた"],
            ),
            ("勝手に入る", &["勝手に", "入る"]),
            ("後継ぎする", &["後継ぎ", "する"]),
            ("なすまん", &["な", "すまん"]),
            ("強いんだね", &["強い", "ん", "だ", "ね"]),
            ("おんなじなんだろ", &["おんなじ", "な", "ん", "だろ"]),
            ("女神様", &["女神", "様"]),
            ("邪推した事柄", &["邪推した", "事柄"]),
            ("邪推してしまう", &["邪推してしまう"]),
            ("良さげかも", &["良さげ", "かも"]),
            ("事故ってます", &["事故ってます"]),
            ("卒倒している", &["卒倒している"]),
            ("卒倒させる", &["卒倒させる"]),
            ("出したいときは", &["出したい", "とき", "は"]),
            ("柔らかさ", &["柔らかさ"]),
            ("次がある", &["次", "が", "ある"]),
            ("のせいですね", &["の", "せい", "です", "ね"]),
            (
                "それただの怪しい人ですし",
                &["それ", "ただ", "の", "怪しい", "人", "です", "し"],
            ),
            ("ごときが知る", &["ごとき", "が", "知る"]),
            ("山にはさまれて", &["山", "に", "はさまれて"]),
            (
                "物がぼんやりとかすんで見える",
                &["物", "が", "ぼんやり", "と", "かすんで", "見える"],
            ),
            (
                "どなた様でございましょうか",
                &["どなた", "様", "でございましょう", "か"],
            ),
            (
                "読んでくださりありがとうございました",
                &["読んで", "くださり", "ありがとうございました"],
            ),
            ("ふざけんな", &["ふざけんな"]),
            ("観終わってた", &["観", "終わってた"]),
            ("意味深終わり", &["意味深", "終わり"]),
            ("今日とて居残りです", &["今日", "とて", "居残り", "です"]),
            ("堪能させていただきます", &["堪能させていただきます"]),
            (
                "わからんからそう思った",
                &["わからん", "から", "そう", "思った"],
            ),
            (
                "うちからそうなっても",
                &["うち", "から", "そう", "なっても"],
            ),
            ("上映会やな", &["上映", "会", "や", "な"]),
            ("以上書いてください", &["以上", "書いてください"]),
            (
                "してしまったのがいまだに忘れられないし",
                &["してしまった", "の", "が", "いまだに", "忘れられない", "し"],
            ),
            ("彼ははんぱじゃなく", &["彼", "は", "はんぱじゃなく"]),
            ("許さないじゃなくてさ", &["許さない", "じゃなくて", "さ"]),
            ("じゃなかったです", &["じゃなかったです"]),
            (
                "彼女は苦しげにうめいて横たわった",
                &["彼女", "は", "苦しげ", "に", "うめいて", "横たわった"],
            ),
            (
                "わたしにはちょっとわかりかねますので",
                &["わたし", "には", "ちょっと", "わかりかねます", "ので"],
            ),
            ("要素はないかと", &["要素", "は", "ない", "か", "と"]),
            ("すごいじゃん", &["すごい", "じゃん"]),
            ("腕をつかまれて路地", &["腕", "を", "つかまれて", "路地"]),
            (
                "別にマイナスにならん",
                &["別に", "マイナス", "に", "ならん"],
            ),
            (
                "遊びばかりはだめだよ",
                &["遊び", "ばかり", "は", "だめ", "だ", "よ"],
            ),
            ("最中でも", &["最中", "でも"]),
            ("小動物好き物好き", &["小動物", "好き", "物好き"]),
            ("知れないですか", &["知れないです", "か"]),
            ("かも知れないですね", &["かも知れない", "です", "ね"]),
            ("匙ですくう", &["匙", "で", "すくう"]),
            ("デカかったクドくない", &["デカかった", "クドくない"]),
            (
                "決めたらしい教われたらしい",
                &["決めたらしい", "教われたらしい"],
            ),
            (
                "臆病なくせにとてもよい仲間だった",
                &["臆病", "な", "くせに", "とても", "よい", "仲間", "だった"],
            ),
            ("あのねあのさ", &["あのね", "あのさ"]),
            (
                "これまでになかったような名優",
                &["これまで", "に", "なかった", "ような", "名優"],
            ),
            ("確かめてちゃんと", &["確かめて", "ちゃんと"]),
            (
                "ことにしましょうってなった",
                &["ことにしましょう", "って", "なった"],
            ),
            ("見てござる", &["見て", "ござる"]),
            (
                "彼がいうことはわけがわからない",
                &["彼", "が", "いう", "こと", "は", "わけがわからない"],
            ),
            (
                "わけのわからないことをくどくど言う",
                &["わけのわからない", "こと", "を", "くどくど", "言う"],
            ),
            ("ごくまれに", &["ごくまれ", "に"]),
            (
                "天をうらんでみたところで始まらない",
                &["天", "を", "うらんで", "みた", "ところで", "始まらない"],
            ),
            ("癒やされたかった", &["癒やされたかった"]),
            ("7時には帰ってきなさい", &["7時", "には", "帰ってきなさい"]),
            ("人はいますか", &["人", "は", "います", "か"]),
            ("トマトづくし", &["トマト", "づくし"]),
            ("見えざる関係性", &["見えざる", "関係性"]),
            ("だめだったら", &["だめ", "だったら"]),
            (
                "万事不都合の無いようにはからってくれ",
                &["万事", "不都合", "の", "無い", "ように", "はからってくれ"],
            ),
            ("ではみなさん", &["では", "みなさん"]),
            ("鉄とはがね", &["鉄", "と", "はがね"]),
            ("抹茶とは", &["抹茶", "とは"]),
            ("工夫がされる", &["工夫", "が", "される"]),
            ("うまいことしたね", &["うまいこと", "した", "ね"]),
            (
                "ことしは新成人１４人のうち８人が避難先などから村の村民会館に集まりました",
                &[
                    "ことし",
                    "は",
                    "新成人",
                    "１４人",
                    "の",
                    "うち",
                    "８人",
                    "が",
                    "避難先",
                    "など",
                    "から",
                    "村",
                    "の",
                    "村民",
                    "会館",
                    "に",
                    "集まりました",
                ],
            ),
            ("鬱が悪化する", &["鬱", "が", "悪化する"]),
            (
                "一部が手に入ればことし１年の願いがかなうとされています",
                &[
                    "一部",
                    "が",
                    "手に入れば",
                    "ことし",
                    "１年",
                    "の",
                    "願い",
                    "が",
                    "かなう",
                    "とされています",
                ],
            ),
            ("汗を流しました", &["汗を流しました"]),
            ("気がついてる", &["気がついてる"]),
            ("ガスがついている", &["ガス", "が", "ついている"]),
            ("再開通", &["再", "開通"]),
            ("謝罪はあったにせよ", &["謝罪", "は", "あった", "にせよ"]),
            ("うそではないにしろ", &["うそ", "ではない", "にしろ"]),
            ("普段着てる服", &["普段", "着てる", "服"]),
            ("エレガントなお洋服", &["エレガント", "な", "お", "洋服"]),
            (
                "老いてなお元気なこと",
                &["老いて", "なお", "元気", "な", "こと"],
            ),
            ("何も口にせぬ", &["何も", "口", "に", "せぬ"]),
            ("切ねぇ", &["切ねぇ"]),
            ("何故人気がある", &["何故", "人気がある"]),
            ("バラしちゃってる", &["バラしちゃってる"]),
            ("気を使わせている", &["気を使わせている"]),
            ("一段上がる", &["一段", "上がる"]),
            ("一段落ちる", &["一段", "落ちる"]),
            ("恐怖ですくむ", &["恐怖", "で", "すくむ"]),
            (
                "全員がたちすくみました",
                &["全員", "が", "たちすくみました"],
            ),
            ("雪がないため", &["雪", "が", "ない", "ため"]),
            ("雪がなく", &["雪", "が", "なく"]),
            ("零れ落ちてる", &["零れ落ちてる"]),
            ("使い物にならんだろ", &["使い物", "に", "ならん", "だろ"]),
            ("私とならんで走った", &["私", "と", "ならんで", "走った"]),
            ("のうえに", &["の", "うえ", "に"]),
            ("皇位についたが", &["皇位", "に", "ついた", "が"]),
            ("疱瘡がついたか", &["疱瘡", "が", "ついた", "か"]),
            ("折りたたみ式ついたて", &["折りたたみ式", "ついたて"]),
            (
                "いろいろな部分をもんだりこすったりすること",
                &[
                    "いろいろ",
                    "な",
                    "部分",
                    "を",
                    "もんだり",
                    "こすったり",
                    "する",
                    "こと",
                ],
            ),
            (
                "たまにはいいもんだよ",
                &["たまに", "は", "いい", "もんだ", "よ"],
            ),
            (
                "歩みをはやめるのだった",
                &["歩み", "を", "はやめる", "の", "だった"],
            ),
            (
                "たばこはやめると誓います",
                &["たばこ", "は", "やめる", "と", "誓います"],
            ),
            (
                "私個人の生活についてとやかくうるさくいうのはやめてください",
                &[
                    "私",
                    "個人",
                    "の",
                    "生活",
                    "について",
                    "とやかく",
                    "うるさく",
                    "いう",
                    "の",
                    "は",
                    "やめてください",
                ],
            ),
            ("こもりがちな人", &["こもりがち", "な", "人"]),
            ("がちなやつ", &["がち", "な", "やつ"]),
            (
                "長くはかからないでしょう",
                &["長く", "は", "かからないでしょう"],
            ),
            (
                "人はいないでしょうね",
                &["人", "は", "いないでしょう", "ね"],
            ),
            ("人はいないですね", &["人", "は", "いないです", "ね"]),
            ("猛者どもの集い", &["猛者", "ども", "の", "集い"]),
            ("うまいかまずいか", &["うまい", "か", "まずい", "か"]),
            ("守衛にとがめられた", &["守衛", "に", "とがめられた"]),
            ("問い合わせがたくさん", &["問い合わせ", "が", "たくさん"]),
            ("楽しみがたくさん", &["楽しみ", "が", "たくさん"]),
            ("ふくろうは", &["ふくろう", "は"]),
            ("語れるもんだな", &["語れる", "もんだ", "な"]),
            ("筋をもんでくれ", &["筋", "を", "もんでくれ"]),
            (
                "いわきからさいたままで",
                &["いわき", "から", "さいたま", "まで"],
            ),
            ("新型コロナウイルス", &["新型コロナウイルス"]),
            ("新型コロナウィルス", &["新型コロナウィルス"]),
            (
                "映画を見るとか食事をするとか",
                &["映画", "を", "見る", "とか", "食事", "を", "する", "とか"],
            ),
            (
                "さもうれしそうに笑う",
                &["さも", "うれしそう", "に", "笑う"],
            ),
            ("出しなに客が来る", &["出しな", "に", "客", "が", "来る"]),
            ("出しながら飛んで", &["出し", "ながら", "飛んで"]),
            ("正直言いたい", &["正直", "言いたい"]),
            (
                "おとめにふさわしい振る舞い",
                &["おとめ", "に", "ふさわしい", "振る舞い"],
            ),
            ("気がないのよ", &["気がない", "の", "よ"]),
            (
                "口論のあげくに殴り合いになった",
                &["口論", "の", "あげく", "に", "殴り合い", "に", "なった"],
            ),
            ("お手数おかけします", &["お手数", "おかけします"]),
            (
                "30分後におかけ直しください",
                &["30分", "後", "に", "お", "かけ直し", "ください"],
            ),
            ("わかりきった", &["わかりきった"]),
            (
                "最良の方法は何だと思いますか",
                &[
                    "最良",
                    "の",
                    "方法",
                    "は",
                    "何",
                    "だ",
                    "と",
                    "思います",
                    "か",
                ],
            ),
            (
                "どうせいやがらせでする",
                &["どうせ", "いやがらせ", "で", "する"],
            ),
            (
                "芝居もどきのせりふを言う",
                &["芝居", "もどき", "の", "せりふ", "を", "言う"],
            ),
            ("がんもどきという食品", &["がんもどき", "という", "食品"]),
            ("落ちこぼれている", &["落ちこぼれている"]),
            ("1話しか見てない", &["1話", "しか", "見てない"]),
            (
                "忙しくてろくに更新もできず",
                &["忙しくて", "ろくに", "更新", "も", "できず"],
            ),
            ("だまってろって", &["だまってろ", "って"]),
            ("しっぽく蕎麦", &["しっぽく", "蕎麦"]),
            (
                "猫はしっぽをぴんとはね上がって歩いた",
                &[
                    "猫",
                    "は",
                    "しっぽ",
                    "を",
                    "ぴんと",
                    "はね上がって",
                    "歩いた",
                ],
            ),
            (
                "物がぴんとはね上がるさま",
                &["物", "が", "ぴんと", "はね上がる", "さま"],
            ),
            ("やる気はない", &["やる気", "は", "ない"]),
            (
                "あけましておめでとうございます",
                &["あけましておめでとうございます"],
            ),
            (
                "おれたちは行くのにおまえたちは行かぬ",
                &[
                    "おれたち",
                    "は",
                    "行く",
                    "のに",
                    "おまえたち",
                    "は",
                    "行かぬ",
                ],
            ),
            ("よろしくおねがいします", &["よろしくおねがいします"]),
            (
                "気を遣ってくれてるのかと思ってました",
                &["気を遣ってくれてる", "のか", "と", "思ってました"],
            ),
            (
                "太陽をかたどったしるし",
                &["太陽", "を", "かたどった", "しるし"],
            ),
            (
                "間違えていらっしゃるのかしら",
                &["間違えて", "いらっしゃる", "の", "かしら"],
            ),
            (
                "ヤツはいそうにないな",
                &["ヤツ", "は", "いそうにない", "な"],
            ),
            ("確認をとっています", &["確認", "を", "とっています"]),
            (
                "人口10万人以上の都市の中で唯一旅客を扱う鉄道駅が存在せず",
                &[
                    "人口",
                    "10万人",
                    "以上",
                    "の",
                    "都市",
                    "の",
                    "中",
                    "で",
                    "唯一",
                    "旅客",
                    "を",
                    "扱う",
                    "鉄道駅",
                    "が",
                    "存在",
                    "せず",
                ],
            ),
            ("だし", &["だ", "し"]),
            ("だしはおいしい", &["だし", "は", "おいしい"]),
            ("だして", &["だして"]),
            ("だしといて", &["だしといて"]),
            ("割り切れたら", &["割り切れたら"]),
            ("あり得なかったり", &["あり得なかったり"]),
            ("代わり映え", &["代わり映え"]),
            (
                "器用なのですぐ上達しますよ",
                &["器用", "なので", "すぐ", "上達します", "よ"],
            ),
            ("おにいちゃん", &["おにいちゃん"]),
            ("動画につまってる", &["動画", "に", "つまってる"]),
            ("出来そう", &["出来そう"]),
            (
                "その上着貸してください",
                &["その", "上着", "貸してください"],
            ),
            ("幸多き", &["幸", "多き"]),
            (
                "きっと気に入っていつかまた来てくれるよ",
                &["きっと", "気に入って", "いつか", "また", "来てくれる", "よ"],
            ),
            (
                "私がいそうな場所知ってたんだから",
                &[
                    "私",
                    "が",
                    "いそう",
                    "な",
                    "場所",
                    "知ってた",
                    "ん",
                    "だから",
                ],
            ),
            ("うまくハメられた", &["うまく", "ハメられた"]),
            ("してるとこだから", &["してる", "とこ", "だから"]),
            ("下記のとおりです", &["下記", "の", "とおり", "です"]),
            ("123ヶ年", &["123ヶ年"]),
            ("そうはいかん", &["そう", "は", "いかん"]),
            (
                "いつなりともお使いなさい",
                &["いつなりと", "も", "お", "使いなさい"],
            ),
            ("よそで待ってて", &["よそ", "で", "待ってて"]),
            ("3つおきの席", &["3つ", "おき", "の", "席"]),
            ("1年おきに", &["1年", "おきに"]),
            ("練習したかいがあって", &["練習した", "かいがあって"]),
            (
                "高いお金を払ったかいがあったと思う",
                &["高い", "お金", "を", "払った", "かいがあった", "と", "思う"],
            ),
            ("養生したかいもなく", &["養生した", "かいもなく"]),
            ("読みがいがある", &["読みがい", "が", "ある"]),
            ("狩りがいのある", &["狩りがい", "の", "ある"]),
            ("懐いている", &["懐いている"]),
            ("カッコよさ", &["カッコよさ"]),
            (
                "上手く案内出来てたらいいんですけど",
                &["上手く", "案内", "出来てたら", "いい", "ん", "です", "けど"],
            ),
            (
                "仲間になりたそうに見ている",
                &["仲間", "に", "なりたそう", "に", "見ている"],
            ),
            (
                "何か問いたそうな口調",
                &["何か", "問いたそう", "な", "口調"],
            ),
            (
                "どんなものにも潮時がある",
                &["どんな", "もの", "にも", "潮時", "が", "ある"],
            ),
            (
                "特化してるというからね",
                &["特化してる", "という", "から", "ね"],
            ),
            ("歩いたぁ", &["歩いた", GAP]),
            ("りばてぃ", &[GAP]),
            ("サウンドトラック", &["サウンドトラック"]),
            ("写真を撮りました", &["写真を撮りました"]),
            ("取り留めの無い", &["取り留めの無い"]),
            ("取り留めも無い", &["取り留めも無い"]),
            ("これへんだ", &["これ", "へん", "だ"]),
            ("おそれたか", &["おそれた", "か"]),
            ("不確かなものに", &["不確か", "な", "もの", "に"]),
            ("まとめていかねばな", &["まとめていかねば", "な"]),
            ("来るからすき", &["来る", "から", "すき"]),
            ("けんかを引分ける", &["けんか", "を", "引分ける"]),
            ("取り計らいましょう", &["取り計らいましょう"]),
            ("一日置いただけで", &["一日", "置いた", "だけ", "で"]),
        ];
        let mut failures: Vec<String> = Vec::new();
        for (input, expected) in cases {
            let result = simple_segment(&ctx, input, None).await.unwrap();
            let actual = segmentation(&result);
            if actual != *expected {
                failures.push(format!(
                    "{:?}: rust={:?} expected={:?}",
                    input, actual, expected
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "{} of {} segmentation cases diverged:\n{}",
            failures.len(),
            cases.len(),
            failures.join("\n")
        );
    }
}
