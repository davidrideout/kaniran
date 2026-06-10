mod find_word_full {
    use crate::dict::path::*;
    use crate::dict::text_classes::ScoreMod;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// A single simple kanji word resolves to one kanji-text, with no
    /// suffix, hiragana, or counter branches.
    #[tokio::test]
    async fn t1_simple_kanji_word() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "区別", false, None).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 1244250);
        assert_eq!(k.text, "区別");
    }

    /// A polysemous word (私) returns multiple kanji-text rows.
    #[tokio::test]
    async fn t2_polysemous_kanji() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "私", false, None).await.unwrap();
        assert_eq!(r.len(), 14);
        for w in &r {
            assert!(matches!(w, KaniWordDispatchEnum::Kanji(_)));
        }
    }

    /// A する-suffix word (勉強する) has no simple match and resolves to
    /// one compound (勉強 + する) via the suru suffix.
    #[tokio::test]
    async fn t3_suru_suffix_via_registered_row() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "勉強する", false, None).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected COMPOUND-TEXT");
        };
        assert_eq!(c.text, "勉強する");
        assert_eq!(c.kana, "べんきょう する");
    }

    /// 我々ら resolves to one compound via the `ra` suffix.
    #[tokio::test]
    async fn t4_ra_suffix_via_registered_row() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "我々ら", false, None).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected COMPOUND-TEXT");
        };
        assert_eq!(c.text, "我々ら");
        assert_eq!(c.kana, "われわれら");
    }

    /// 食べてる resolves to one compound via the teiru suffix: kanji
    /// primary 食べて plus kana auxiliary いる.
    #[tokio::test]
    async fn t5_teiru_suffix_compound() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "食べてる", false, None).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected Compound, got {:?}", r[0]);
        };
        assert_eq!(c.text, "食べてる");
        assert_eq!(c.kana, "たべてる");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
        assert!(c.score_base.is_none());
        let KaniWordDispatchEnum::Kanji(primary) = &*c.primary else {
            panic!("expected Kanji primary, got {:?}", c.primary);
        };
        assert_eq!(primary.seq, 10092233);
        assert_eq!(primary.text, "食べて");
        assert_eq!(c.words.len(), 2);
        let KaniWordDispatchEnum::Kanji(w0) = &c.words[0] else {
            panic!("expected Kanji words[0]");
        };
        assert_eq!(w0.seq, 10092233);
        let KaniWordDispatchEnum::Kana(w1) = &c.words[1] else {
            panic!("expected Kana words[1]");
        };
        assert_eq!(w1.seq, 1577980);
        assert_eq!(w1.text, "いる");
    }

    /// An unmatchable string returns nothing — no simple match, no suffix
    /// expansion.
    #[tokio::test]
    async fn t6_no_match() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "xyzabc", false, None).await.unwrap();
        assert!(r.is_empty());
    }

    /// With as-hiragana on, a katakana word that already has a kana row
    /// returns just that row — the hiragana fallback excludes the same
    /// seq, so no proxies are added.
    #[tokio::test]
    async fn t7_as_hiragana_with_existing_kana_match() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "ジャバスクリプト", true, None)
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kana(k) = &r[0] else {
            panic!("expected KANA-TEXT");
        };
        assert_eq!(k.seq, 2302400);
    }

    /// With as-hiragana on, ハイ returns its own kana row plus 13 proxy
    /// rows wrapping the はい readings.
    #[tokio::test]
    async fn t8_as_hiragana_with_proxy_fallback() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "ハイ", true, None).await.unwrap();
        assert_eq!(r.len(), 14);
        let kana_count = r
            .iter()
            .filter(|w| matches!(w, KaniWordDispatchEnum::Kana(_)))
            .count();
        let proxy_count = r
            .iter()
            .filter(|w| matches!(w, KaniWordDispatchEnum::Proxy(_)))
            .count();
        assert_eq!(kana_count, 1);
        assert_eq!(proxy_count, 13);
    }

    /// With auto counter detection, 三本 returns the kanji word plus two
    /// counter rows.
    #[tokio::test]
    async fn t9_counter_auto_with_simple_match() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "三本", false, Some(CounterArg::Auto))
            .await
            .unwrap();
        assert_eq!(r.len(), 3);
        assert!(matches!(r[0], KaniWordDispatchEnum::Kanji(_)));
        assert!(matches!(r[1], KaniWordDispatchEnum::Counter(_)));
        assert!(matches!(r[2], KaniWordDispatchEnum::Counter(_)));
    }

    /// With an explicit counter index, 5本 returns two counter rows (the
    /// number "5" and the unit "本").
    #[tokio::test]
    async fn t10_counter_explicit_index() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "5本", false, Some(CounterArg::At(1)))
            .await
            .unwrap();
        assert_eq!(r.len(), 2);
        for w in &r {
            assert!(matches!(w, KaniWordDispatchEnum::Counter(_)));
        }
    }

    /// With auto counter detection but no number group (区別), only the
    /// kanji word is returned — the counter branch contributes nothing.
    #[tokio::test]
    async fn t11_counter_auto_no_number_group() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "区別", false, Some(CounterArg::Auto))
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
        assert!(matches!(r[0], KaniWordDispatchEnum::Kanji(_)));
    }

    /// An over-length input returns nothing: the max-word-length gate
    /// short-circuits the simple-word path, and the suffix branch finds no
    /// cache hit on this 51-character hiragana run.
    #[tokio::test]
    async fn t12_over_length_short_circuit() {
        let ctx = ctx().await;
        let long = "あいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもやゆよらりるれろわをんがぎぐげござ";
        let r = find_word_full(&ctx, long, false, None).await.unwrap();
        assert!(r.is_empty());
    }
}

mod join_substring_words_star_ {
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::*;
    // Run with `cargo test ... -- --test-threads=1` per the DB-test
    // convention.

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// `(start, end, segment-count)` shape of the result.
    fn shape(result: &[(usize, usize, Vec<Segment>)]) -> Vec<(usize, usize, usize)> {
        result
            .iter()
            .map(|(s, e, segs)| (*s, *e, segs.len()))
            .collect()
    }

    /// A kanji run (日本語) accumulates sequential kanji-break positions
    /// across reachable starts, deduped keep-last, giving kanji-break `(2 1)`.
    #[tokio::test]
    async fn nihongo_kanji_run() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "日本語").await.unwrap();
        assert_eq!(
            shape(&result),
            vec![(0, 1, 4), (0, 2, 1), (0, 3, 1), (1, 2, 2), (2, 3, 2)]
        );
        assert_eq!(kanji_break, vec![2, 1]);
    }

    /// For 特大, start=1 is not reachable (not in `ends`), so its segment
    /// does not contribute to kanji-break, leaving `(1)`.
    #[tokio::test]
    async fn tokudai_start_not_reachable() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "特大").await.unwrap();
        assert_eq!(shape(&result), vec![(0, 2, 1), (1, 2, 5)]);
        assert_eq!(kanji_break, vec![1]);
    }

    /// In 私は学生です the slice "です" is in the force-kanji-break set
    /// (adds position 5) and "学生" contributes the sequential position 3,
    /// giving kanji-break `(5 3)`.
    #[tokio::test]
    async fn watashi_force_kanji_break_desu() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "私は学生です")
            .await
            .unwrap();
        assert_eq!(
            shape(&result),
            vec![
                (0, 1, 14),
                (1, 2, 11),
                (2, 3, 1),
                (2, 4, 2),
                (3, 4, 7),
                (4, 5, 4),
                (4, 6, 2),
                (5, 6, 10),
            ]
        );
        assert_eq!(kanji_break, vec![5, 3]);
    }

    /// In 一日置く the slice "日置" is in the no-kanji-break set, so the
    /// sequential position 2 it would contribute is suppressed —
    /// kanji-break is `(1)`, not `(2 1)`.
    #[tokio::test]
    async fn ichinichi_no_kanji_break() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "一日置く").await.unwrap();
        assert_eq!(
            shape(&result),
            vec![
                (0, 1, 6),
                (0, 2, 5),
                (1, 2, 4),
                (1, 3, 1),
                (2, 4, 1),
                (3, 4, 8)
            ]
        );
        assert_eq!(kanji_break, vec![1]);
        // The [1 3] "日置" slice is present but suppresses its break.
        assert!(result.iter().any(|(s, e, _)| *s == 1 && *e == 3));
    }

    /// For コーヒー the katakana group spans 0..4, so the whole slice
    /// looks up as hiragana and yields the kana row; the sticky position
    /// 1 is absent from every slice.
    #[tokio::test]
    async fn coffee_as_hiragana_and_sticky() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "コーヒー").await.unwrap();
        assert_eq!(shape(&result), vec![(0, 4, 1), (3, 4, 1)]);
        assert!(kanji_break.is_empty());
        // No slice starts or ends at the sticky position 1.
        assert!(!result.iter().any(|(s, e, _)| *s == 1 || *e == 1));
        // [0 4] is the existing コーヒー kana row (as-hiragana path).
        let (_, _, segs) = result.iter().find(|(s, e, _)| *s == 0 && *e == 4).unwrap();
        assert!(matches!(segs[0].word, KaniWordDispatchEnum::Kana(_)));
    }

    /// For 5本 the number group drives counter detection: "5" yields a
    /// number-text, "5本" yields two counter rows, and "本" is two plain
    /// kanji-text.
    #[tokio::test]
    async fn counter_number_group() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "5本").await.unwrap();
        assert_eq!(shape(&result), vec![(0, 1, 1), (0, 2, 2), (1, 2, 2)]);
        assert!(kanji_break.is_empty());
        let (_, _, num) = result.iter().find(|(s, e, _)| *s == 0 && *e == 1).unwrap();
        assert!(matches!(num[0].word, KaniWordDispatchEnum::Counter(_)));
        let (_, _, cnt) = result.iter().find(|(s, e, _)| *s == 0 && *e == 2).unwrap();
        assert!(cnt
            .iter()
            .all(|seg| matches!(seg.word, KaniWordDispatchEnum::Counter(_))));
        let (_, _, hon) = result.iter().find(|(s, e, _)| *s == 1 && *e == 2).unwrap();
        assert!(hon
            .iter()
            .all(|seg| matches!(seg.word, KaniWordDispatchEnum::Kanji(_))));
    }

    /// In やっぱり the sokuon makes position 2 sticky, so no slice starts
    /// or ends there; kanji-break is empty for the all-kana input.
    #[tokio::test]
    async fn yappari_sokuon_sticky() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "やっぱり").await.unwrap();
        assert_eq!(
            shape(&result),
            vec![(0, 1, 9), (0, 3, 1), (0, 4, 1), (1, 3, 1), (3, 4, 8)]
        );
        assert!(kanji_break.is_empty());
        assert!(!result.iter().any(|(s, e, _)| *s == 2 || *e == 2));
    }

    /// Empty input gives an empty result and an empty kanji-break.
    #[tokio::test]
    async fn empty_string() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "").await.unwrap();
        assert!(result.is_empty());
        assert!(kanji_break.is_empty());
    }

    /// Deduplication keeps the last occurrence of each value: `(1 2 1)` → `(2 1)`.
    #[test]
    fn remove_duplicates_keeps_last() {
        assert_eq!(remove_duplicates(&[1, 2, 1]), vec![2, 1]);
        assert_eq!(remove_duplicates(&[5, 3]), vec![5, 3]);
        assert_eq!(remove_duplicates(&[]), Vec::<usize>::new());
        assert_eq!(remove_duplicates(&[2, 2, 2]), vec![2]);
    }
}

mod join_substring_words {
    use crate::dict::path::*;
    // Run with `cargo test ... -- --test-threads=1` per the DB-test
    // convention.

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// Per segment-list: `(start, end, matches, [scores high-to-low])`.
    /// Score values are deterministic (calc-score); order among equal
    /// scores can rotate with `find-word`'s unordered SQL, so scores are
    /// compared as a sorted-descending list.
    fn summarize(sls: &[SegmentList]) -> Vec<(usize, usize, usize, Vec<i32>)> {
        sls.iter()
            .map(|sl| {
                let mut scores: Vec<i32> =
                    sl.segments.iter().map(|seg| seg.score.unwrap()).collect();
                scores.sort_unstable_by(|a, b| b.cmp(a));
                (sl.start, sl.end, sl.matches, scores)
            })
            .collect()
    }

    /// 日本語 yields 5 segment-lists; kanji-break `(2 1)` drives the
    /// per-slice kanji-break, and `[0 1]` keeps 2 of its 4 matches after
    /// culling.
    #[tokio::test]
    async fn nihongo() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "日本語").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![
                (0, 1, 4, vec![12, 8]),
                (0, 2, 1, vec![104]),
                (0, 3, 1, vec![1054]),
                (1, 2, 2, vec![8, 6]),
                (2, 3, 2, vec![18]),
            ]
        );
    }

    /// 特大 yields 2 segment-lists; `[1 2]` has 5 matches but a single
    /// surviving segment after cutoff and culling.
    #[tokio::test]
    async fn tokudai() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "特大").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![(0, 2, 1, vec![208]), (1, 2, 5, vec![18])]
        );
    }

    /// 私は学生です yields 7 segment-lists: です forces a kanji-break and
    /// 学生 adds a sequential one; `[0 1]` keeps 3 of 14 私 readings and
    /// `[3 4]` keeps 3 of 7.
    #[tokio::test]
    async fn watashi_wa_gakusei_desu() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "私は学生です").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![
                (0, 1, 14, vec![25, 16, 16]),
                (1, 2, 11, vec![11]),
                (2, 3, 1, vec![8]),
                (2, 4, 2, vec![325]),
                (3, 4, 7, vec![13, 13, 8]),
                (4, 5, 4, vec![11]),
                (4, 6, 2, vec![64]),
            ]
        );
    }

    /// 5本 drives the counter path: "5" is a number-text scoring exactly
    /// at the cutoff (5), and "5本" yields two counter rows.
    #[tokio::test]
    async fn counter_5hon() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "5本").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![
                (0, 1, 1, vec![5]),
                (0, 2, 2, vec![128, 88]),
                (1, 2, 2, vec![16, 11]),
            ]
        );
    }

    /// ねこー ends with a long-vowel mark, so the slice ending one short
    /// of the full length (ねこ) is still treated as final.
    #[tokio::test]
    async fn neko_lw_final_branch() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "ねこー").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![(0, 1, 8, vec![6]), (0, 2, 1, vec![16])]
        );
    }

    /// サッカー ends with a long-vowel mark; its sole slice spans the
    /// full length so it is final, and 3 matches collapse to a single
    /// kana row.
    #[tokio::test]
    async fn sakka_lw() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "サッカー").await.unwrap();
        assert_eq!(summarize(&sls), vec![(0, 4, 3, vec![80])]);
    }

    /// Empty input yields no segment-lists.
    #[tokio::test]
    async fn empty() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "").await.unwrap();
        assert!(sls.is_empty());
    }

    /// Checks the matched word text at the slice level, not just the score
    /// shape: the whole-string slice of 日本語 is the single 日本語 entry.
    #[tokio::test]
    async fn slice_word_text() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "日本語").await.unwrap();
        let whole = sls
            .iter()
            .find(|sl| sl.start == 0 && sl.end == 3)
            .unwrap();
        assert_eq!(whole.segments.len(), 1);
        let mut seg = (*whole.segments[0]).clone();
        assert_eq!(seg.get_text(), "日本語");
    }
}

mod substring_index {
    use crate::dict::path::*;
    // Run with `-- --test-threads=1` per the DB-test convention.

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// Per index entry: `(key, sl.start, sl.end, n_segments)`, sorted by
    /// key so the unordered hash compares deterministically.
    fn summarize(
        index: &HashMap<(usize, usize), SegmentList>,
    ) -> Vec<((usize, usize), usize, usize, usize)> {
        let mut rows: Vec<((usize, usize), usize, usize, usize)> = index
            .iter()
            .map(|(key, sl)| (*key, sl.start, sl.end, sl.segments.len()))
            .collect();
        rows.sort_unstable();
        rows
    }

    /// 日本語 indexes to 5 entries; each value's start/end equals its key
    /// and segment counts match join-substring-words.
    #[tokio::test]
    async fn nihongo() {
        let ctx = ctx().await;
        let index = substring_index(&ctx, "日本語").await.unwrap();
        assert_eq!(
            summarize(&index),
            vec![
                ((0, 1), 0, 1, 2),
                ((0, 2), 0, 2, 1),
                ((0, 3), 0, 3, 1),
                ((1, 2), 1, 2, 2),
                ((2, 3), 2, 3, 1),
            ]
        );
    }

    /// 特大 indexes to 2 entries.
    #[tokio::test]
    async fn tokudai() {
        let ctx = ctx().await;
        let index = substring_index(&ctx, "特大").await.unwrap();
        assert_eq!(
            summarize(&index),
            vec![((0, 2), 0, 2, 1), ((1, 2), 1, 2, 1)]
        );
    }

    /// 5本 indexes to 3 entries; the counter slice `(0 2)` keeps 2 segments.
    #[tokio::test]
    async fn counter_5hon() {
        let ctx = ctx().await;
        let index = substring_index(&ctx, "5本").await.unwrap();
        assert_eq!(
            summarize(&index),
            vec![((0, 1), 0, 1, 1), ((0, 2), 0, 2, 2), ((1, 2), 1, 2, 2)]
        );
    }

    /// Empty input gives an empty index.
    #[tokio::test]
    async fn empty() {
        let ctx = ctx().await;
        let index = substring_index(&ctx, "").await.unwrap();
        assert!(index.is_empty());
    }
}

mod top_array_class {
    use crate::dict::path::*;

    #[test]
    fn new_preallocates_limit_with_nones() {
        let ta = TopArray::new(5);
        assert_eq!(ta.array.len(), 5);
        assert!(ta.array.iter().all(|x| x.is_none()));
        assert_eq!(ta.count, 0);
    }
}

mod gap_penalty {
    use crate::dict::path::*;

    #[test]
    fn matches_repl() {
        assert_eq!(gap_penalty(0, 0), 0);
        assert_eq!(gap_penalty(0, 3), -1500);
        assert_eq!(gap_penalty(7, 9), -1000);
        assert_eq!(gap_penalty(10, 10), 0);
        assert_eq!(gap_penalty(5, 2), 1500);
    }
}

mod get_seg_initial {
    use crate::dict::conj::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::dao::SimpleText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::path::*;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};

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

    fn info_with_seq_set(seq_set: Vec<i32>) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: vec![],
            seq_set,
            conj: vec![] as Vec<ConjData>,
            common: None,
            score_info: KaniScoreInfo {
                prop_score: 0,
                kanji_break: vec![],
                use_length_bonus: 0,
                split_info: KaniSplitInfo::None,
            },
            kpcl: (false, false, false, false),
        }
    }

    fn seg(start: usize, end: usize, seq_set: Vec<i32>) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(info_with_seq_set(seq_set)),
            top: None,
            text: Some(String::new()),
        }
    }

    fn lite_sl(
        start: usize,
        end: usize,
        matches: usize,
        segments: Vec<Segment>,
    ) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches,
        }))
    }

    fn assert_seq_sets(actual: &KaniLiteSegmentList, expected: &[Vec<i32>]) {
        assert_eq!(actual.segments.len(), expected.len());
        for (i, exp) in expected.iter().enumerate() {
            assert_eq!(&actual.segments[i].seq_set, exp, "segments[{}]", i);
        }
    }

    #[test]
    fn a1_empty_segment_list_returns_passthrough() {
        let r = lite_sl(0, 0, 0, vec![]);
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].start, 0);
        assert_eq!(got[0].end, 0);
        assert!(got[0].segments.is_empty());
    }

    #[test]
    fn a2_seq_not_in_any_segfilter_returns_one_unchanged() {
        let r = lite_sl(0, 2, 0, vec![seg(0, 2, vec![999])]);
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        assert_seq_sets(&got[0], &[vec![999]]);
    }

    #[test]
    fn a3_aux_verb_only_seg_yields_zero_splits() {
        let r = lite_sl(0, 2, 0, vec![seg(0, 2, vec![1342560])]);
        let got = get_seg_initial(&r);
        assert!(got.is_empty());
    }

    #[test]
    fn a4_matches_field_carries_through_unchanged() {
        let r = lite_sl(0, 2, 7, vec![seg(0, 2, vec![999])]);
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].matches, 7);
        assert_seq_sets(&got[0], &[vec![999]]);
    }

    #[test]
    fn a5_mixed_aux_and_normal_yields_filtered_subset() {
        let r = lite_sl(
            0,
            2,
            0,
            vec![seg(0, 2, vec![1342560]), seg(0, 2, vec![999])],
        );
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].segments.len(), 1);
        assert!(got[0].segments[0].seq_set.contains(&999));
        assert!(!got[0].segments[0].seq_set.contains(&1342560));
    }
}

mod get_seg_splits {
    use crate::dict::conj::ConjData;
    use crate::dict::dao::ConjProp;
    use crate::dict::dao::KanaText;
    use crate::dict::dao::SimpleText;
    use crate::dict::grammar::synergy::Synergy;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::path::*;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};

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

    fn cdata(conj_type: i32) -> ConjData {
        ConjData {
            seq: None,
            from: None,
            via: None,
            prop: Some(ConjProp {
                id: 0,
                conj_id: 0,
                conj_type,
                pos: String::new(),
                neg: None,
                fml: None,
            }),
            src_map: vec![],
        }
    }

    fn info(
        seq_set: Vec<i32>,
        conj: Vec<ConjData>,
        posi: Vec<&str>,
        kpcl: (bool, bool, bool, bool),
    ) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: posi.into_iter().map(String::from).collect(),
            seq_set,
            conj,
            common: None,
            score_info: KaniScoreInfo {
                prop_score: 0,
                kanji_break: vec![],
                use_length_bonus: 0,
                split_info: KaniSplitInfo::None,
            },
            kpcl,
        }
    }

    fn seg(start: usize, end: usize, info: KaniSegmentInfo, text: &str) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(info),
            top: None,
            text: Some(text.to_string()),
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    fn unwrap_sl(elem: &KaniLitePathElement) -> &Arc<KaniLiteSegmentList> {
        match elem {
            KaniLitePathElement::SegmentList(sl) => sl,
            other => panic!("expected SegmentList, got {:?}", other),
        }
    }

    fn unwrap_synergy(elem: &KaniLitePathElement) -> &Synergy {
        match elem {
            KaniLitePathElement::Synergy(s) => s,
            other => panic!("expected Synergy, got {:?}", other),
        }
    }

    #[test]
    fn a_no_penalty_no_synergy_yields_one_fallback_outer() {
        let l = lite_sl(
            0,
            3,
            vec![seg(
                0,
                3,
                info(vec![9999], vec![], vec![], (true, false, false, false)),
                "abc",
            )],
        );
        let r = lite_sl(
            3,
            6,
            vec![seg(
                3,
                6,
                info(vec![8888], vec![], vec![], (true, false, false, false)),
                "def",
            )],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].len(), 2);
        assert_eq!(unwrap_sl(&got[0][0]).start, 3);
        assert_eq!(unwrap_sl(&got[0][0]).end, 6);
        assert_eq!(unwrap_sl(&got[0][1]).start, 0);
        assert_eq!(unwrap_sl(&got[0][1]).end, 3);
    }

    #[test]
    fn b_penalty_short_only_yields_one_penalty_outer() {
        let l = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                info(vec![9999], vec![], vec![], (false, false, false, false)),
                "あ",
            )],
        );
        let r = lite_sl(
            3,
            4,
            vec![seg(
                3,
                4,
                info(vec![8888], vec![], vec![], (false, false, false, false)),
                "い",
            )],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].len(), 3);
        assert_eq!(unwrap_sl(&got[0][0]).start, 3);
        assert_eq!(unwrap_sl(&got[0][0]).end, 4);
        let syn = unwrap_synergy(&got[0][1]);
        assert_eq!(syn.description.as_deref(), Some("short"));
        assert_eq!(syn.score, -9);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 3);
        assert_eq!(unwrap_sl(&got[0][2]).start, 0);
        assert_eq!(unwrap_sl(&got[0][2]).end, 1);
    }

    #[test]
    fn c_synergy_no_adjectives_only_yields_fallback_plus_synergy() {
        let l = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                info(vec![], vec![], vec!["adj-no"], (true, false, false, false)),
                "x",
            )],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(
                1,
                2,
                info(vec![1469800], vec![], vec![], (false, false, false, false)),
                "y",
            )],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].len(), 2);
        assert_eq!(unwrap_sl(&got[0][0]).start, 1);
        assert_eq!(unwrap_sl(&got[0][1]).start, 0);
        assert_eq!(got[1].len(), 3);
        let syn = unwrap_synergy(&got[1][1]);
        assert_eq!(syn.description.as_deref(), Some("no-adjective"));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
    }

    #[test]
    fn d_aux_verb_segfilter_split_yields_two_fallback_outers() {
        let l = lite_sl(
            0,
            2,
            vec![
                seg(
                    0,
                    2,
                    info(
                        vec![],
                        vec![cdata(13)],
                        vec![],
                        (false, false, false, false),
                    ),
                    "x1",
                ),
                seg(
                    0,
                    2,
                    info(vec![], vec![cdata(3)], vec![], (false, false, false, false)),
                    "x2",
                ),
            ],
        );
        let r = lite_sl(
            2,
            4,
            vec![
                seg(
                    2,
                    4,
                    info(vec![1342560], vec![], vec![], (false, false, false, false)),
                    "y1",
                ),
                seg(
                    2,
                    4,
                    info(vec![999], vec![], vec![], (false, false, false, false)),
                    "y2",
                ),
            ],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 2);
        for outer in &got {
            assert_eq!(outer.len(), 2);
            assert_eq!(unwrap_sl(&outer[0]).start, 2);
            assert_eq!(unwrap_sl(&outer[1]).start, 0);
        }
    }

    #[test]
    fn e_non_adjacent_blocks_synergy_keeps_fallback() {
        let l = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                info(vec![], vec![], vec!["adj-no"], (true, false, false, false)),
                "x",
            )],
        );
        let r = lite_sl(
            3,
            4,
            vec![seg(
                3,
                4,
                info(vec![1469800], vec![], vec![], (false, false, false, false)),
                "y",
            )],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].len(), 2);
        assert_eq!(unwrap_sl(&got[0][0]).start, 3);
        assert_eq!(unwrap_sl(&got[0][1]).start, 0);
    }

    #[test]
    fn f_penalty_semi_final_plus_synergy_no_adjectives() {
        let l = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                info(
                    vec![2029110],
                    vec![],
                    vec!["adj-no"],
                    (true, false, false, false),
                ),
                "x",
            )],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(
                1,
                2,
                info(vec![1469800], vec![], vec![], (false, false, false, false)),
                "y",
            )],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].len(), 3);
        let syn0 = unwrap_synergy(&got[0][1]);
        assert_eq!(syn0.description.as_deref(), Some("semi-final not final"));
        assert_eq!(syn0.score, -15);
        assert_eq!(got[1].len(), 3);
        let syn1 = unwrap_synergy(&got[1][1]);
        assert_eq!(syn1.description.as_deref(), Some("no-adjective"));
        assert_eq!(syn1.score, 15);
    }
}

mod find_best_path {
    use crate::dict::path::*;
    // These cover the empty-input cases. The DB-dependent non-empty paths
    // (outer loop, get-seg-initial, get-seg-splits accumulation) are
    // covered by the audit binary at `audit/dict/find_best_path_test.rs`.

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[tokio::test]
    async fn empty_input_length_5_default_limit() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 5, None).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty(), "initial gap-seed has empty payload");
        assert_eq!(result[0].1, -2500);
    }

    #[tokio::test]
    async fn empty_input_length_0() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 0, None).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty());
        assert_eq!(result[0].1, 0);
    }

    #[tokio::test]
    async fn empty_input_length_1_limit_3() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 1, Some(3)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty());
        assert_eq!(result[0].1, -500);
    }

    #[tokio::test]
    async fn empty_input_length_1_limit_1() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 1, Some(1)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty());
        assert_eq!(result[0].1, -500);
    }
}
