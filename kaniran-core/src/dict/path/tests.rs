mod find_word_full {
    use crate::dict::path::*;
    use crate::dict::text_classes::ScoreMod;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// REPL: `(find-word-full "区別")` → 1 KANJI-TEXT (seq=1244250).
    /// Single simple-text, no suffix / hiragana / counter branches.
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

    /// REPL: `(find-word-full "私")` → 14 KANJI-TEXT rows (polysemous
    /// 私). Exercises multi-row simple-words.
    #[tokio::test]
    async fn t2_polysemous_kanji() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "私", false, None).await.unwrap();
        assert_eq!(r.len(), 14);
        for w in &r {
            assert!(matches!(w, KaniWordDispatchEnum::Kanji(_)));
        }
    }

    /// REPL: `(find-word-full "勉強する")` → 1 COMPOUND-TEXT.
    /// simple-words for 勉強する is empty; suffix-suru fires through
    /// the partial `*suffix-list*` (suru row registered) and produces
    /// 1 compound (勉強+する).
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

    /// REPL: `(find-word-full "我々ら")` → 1 COMPOUND-TEXT via the
    /// `ra` suffix row.
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

    /// REPL: `(find-word-full "食べてる")` → 1 COMPOUND-TEXT
    /// text="食べてる" kana="たべてる" via `suffix-teiru`. primary =
    /// KANJI-TEXT 食べて (seq=10092233), words = (primary, KANA-TEXT
    /// いる seq=1577980), score_mod=3, score_base=nil.
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

    /// REPL: `(find-word-full "xyzabc")` → NIL. No simple-text, no
    /// suffix expansion via the cache.
    #[tokio::test]
    async fn t6_no_match() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "xyzabc", false, None).await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-full "ジャバスクリプト" :as-hiragana t)` → 1
    /// (the existing kana_text row 2302400; the hiragana fallback
    /// excludes the same seq, so no proxies added).
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

    /// REPL: `(find-word-full "ハイ" :as-hiragana t)` → 14:
    ///   1 KANA-TEXT (the existing ハイ row) + 13 PROXY-TEXT (the
    ///   13 はい kana_text root rows wrapped as proxies).
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

    /// REPL: `(find-word-full "三本" :counter :auto)` → 3:
    ///   1 KANJI-TEXT (existing 三本) + 1 COUNTER-TEXT + 1
    ///   COUNTER-HIFUMI. Exercises the `:auto` branch through
    ///   `consecutive-char-groups`.
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

    /// REPL: `(find-word-full "5本" :counter 1)` → 2 COUNTER-TEXT
    /// (number text "5", counter unit "本"). Integer-index branch;
    /// simple-words is empty so `:unique` resolves to T.
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

    /// REPL: `(find-word-full "区別" :counter :auto)` → 1 (just the
    /// kanji-text; `consecutive-char-groups :number` returns NIL for
    /// 区別 → counter branch contributes nothing).
    #[tokio::test]
    async fn t11_counter_auto_no_number_group() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "区別", false, Some(CounterArg::Auto))
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
        assert!(matches!(r[0], KaniWordDispatchEnum::Kanji(_)));
    }

    /// REPL: `(find-word-full <long>)` → 0 (full result, not just the
    /// `find-word` branch). The `*max-word-length*` gate inside
    /// `find-word` short-circuits the simple-words path; the
    /// `find-word-suffix` branch still runs but finds no cache hit on
    /// this specific 51-char hiragana run — REPL-verified against
    /// both the random-hiragana string below and a realistic
    /// over-length sentence.
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
    // Every assertion is REPL-verified against the .103 SBCL via
    // `(ichiran/dict::join-substring-words* …)` (2026-05-23 probe runs).
    // Run with `cargo test ... -- --test-threads=1` per the DB-test
    // convention.

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// `(start, end, segment-count)` shape of the result — the
    /// loop-bound / sticky / find-word behavior that the function owns.
    fn shape(result: &[(usize, usize, Vec<Segment>)]) -> Vec<(usize, usize, usize)> {
        result
            .iter()
            .map(|(s, e, segs)| (*s, *e, segs.len()))
            .collect()
    }

    /// REPL `(join-substring-words* "日本語")`:
    /// `[0 1] n=4 [0 2] n=1 [0 3] n=1 [1 2] n=2 [2 3] n=2`,
    /// kanji-break `(2 1)` — sequential-kanji-positions accumulated
    /// across reachable starts, deduped keep-last.
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

    /// REPL `(join-substring-words* "特大")`:
    /// `[0 2] n=1 [1 2] n=5`, kanji-break `(1)`. start=1 is not in
    /// `ends` so its segment does not add to kanji-break.
    #[tokio::test]
    async fn tokudai_start_not_reachable() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "特大").await.unwrap();
        assert_eq!(shape(&result), vec![(0, 2, 1), (1, 2, 5)]);
        assert_eq!(kanji_break, vec![1]);
    }

    /// REPL `(join-substring-words* "私は学生です")`:
    /// kanji-break `(5 3)`. The `[4 6]` slice "です" is in
    /// *force-kanji-break* → iota over its interior (position 5); the
    /// `[2 4]` slice "学生" contributes the sequential position 3.
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

    /// REPL `(join-substring-words* "一日置く")`: the `[1 3]` slice
    /// "日置" is in *no-kanji-break*, so the sequential position 2 it
    /// would otherwise contribute is suppressed — kanji-break is `(1)`,
    /// not `(2 1)`.
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

    /// REPL `(join-substring-words* "コーヒー")` (sticky=(1)): the
    /// katakana group spans 0..4, so the `[0 4]` slice runs
    /// find-word-full with as-hiragana=T and yields the kana row.
    /// start=1 and end=1 are sticky → absent.
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

    /// REPL `(join-substring-words* "5本")`: the number group at 0..1
    /// drives the :counter argument. `[0 1]` "5" yields a NUMBER-TEXT;
    /// `[0 2]` "5本" yields COUNTER-TEXT + COUNTER-HIFUMI; `[1 2]` "本"
    /// is two plain KANJI-TEXT.
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

    /// REPL `(join-substring-words* "やっぱり")` (sticky=(2)): the
    /// sokuon makes position 2 sticky, so no slice starts or ends
    /// there. kanji-break empty (all-kana input).
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

    /// REPL `(join-substring-words* "")`: empty input → empty result,
    /// empty kanji-break (the outer loop range is empty).
    #[tokio::test]
    async fn empty_string() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "").await.unwrap();
        assert!(result.is_empty());
        assert!(kanji_break.is_empty());
    }

    /// `remove-duplicates` keep-last semantics, pinned directly:
    /// REPL `(remove-duplicates '(1 2 1))` → `(2 1)`.
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
    // Every assertion is REPL-verified against the .103 SBCL via
    // `(ichiran/dict::join-substring-words …)` (2026-05-23 probe runs).
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

    /// REPL `(join-substring-words "日本語")`: 5 segment-lists.
    /// kanji-break is `(2 1)`, so `[1 2]`'s kb is the two-element
    /// reverse-order case `(1 0)`. `[0 1]` keeps 2 of its 4 matches.
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

    /// REPL `(join-substring-words "特大")`: 2 segment-lists. `[1 2]`
    /// has matches=5 but a single surviving segment after cutoff/cull.
    #[tokio::test]
    async fn tokudai() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "特大").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![(0, 2, 1, vec![208]), (1, 2, 5, vec![18])]
        );
    }

    /// REPL `(join-substring-words "私は学生です")`: 7 segment-lists.
    /// です is in *force-kanji-break*, 学生 contributes a sequential
    /// break; `[0 1]` keeps 3 of 14 私 readings, `[3 4]` keeps 3 of 7.
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

    /// REPL `(join-substring-words "5本")`: the number group drives the
    /// counter path. `[0 1]` "5" is a NUMBER-TEXT scoring exactly at the
    /// cutoff (5); `[0 2]` "5本" yields COUNTER-TEXT + COUNTER-HIFUMI.
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

    /// REPL `(join-substring-words "ねこー")`: ends-with-lw is T. The
    /// `[0 2]` "ねこ" slice ends at length-1 (=2), so its `:final` is the
    /// `(and ends-with-lw (= end (1- length)))` branch.
    #[tokio::test]
    async fn neko_lw_final_branch() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "ねこー").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![(0, 1, 8, vec![6]), (0, 2, 1, vec![16])]
        );
    }

    /// REPL `(join-substring-words "サッカー")`: ends-with-lw is T, sole
    /// slice `[0 4]` ends at length so `:final` is T via the first
    /// disjunct. matches=3 collapses to a single kana row.
    #[tokio::test]
    async fn sakka_lw() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "サッカー").await.unwrap();
        assert_eq!(summarize(&sls), vec![(0, 4, 3, vec![80])]);
    }

    /// REPL `(join-substring-words "")`: empty input → no segment-lists.
    #[tokio::test]
    async fn empty() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "").await.unwrap();
        assert!(sls.is_empty());
    }

    /// Word identity at the slice level: `[0 3]` of 日本語 is the single
    /// 日本語 entry (seq 1464530), `[2 4]` of 私は学生です is 学生
    /// (seq 1270790-class kanji row). Checks the matched word text, not
    /// just the score shape.
    #[tokio::test]
    async fn slice_word_text() {
        let ctx = ctx().await;
        let mut sls = join_substring_words(&ctx, "日本語").await.unwrap();
        let whole = sls
            .iter_mut()
            .find(|sl| sl.start == 0 && sl.end == 3)
            .unwrap();
        assert_eq!(whole.segments.len(), 1);
        assert_eq!(whole.segments[0].get_text(), "日本語");
    }
}

mod substring_index {
    use crate::dict::path::*;
    // Every assertion is REPL-verified against the .103 SBCL via
    // `(ichiran/dict::substring-index …)` (2026-05-25 probe).
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

    /// REPL `(substring-index "日本語")`: 5 entries; each value's
    /// start/end equals its key, segment counts match join-substring-words.
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

    /// REPL `(substring-index "特大")`: 2 entries.
    #[tokio::test]
    async fn tokudai() {
        let ctx = ctx().await;
        let index = substring_index(&ctx, "特大").await.unwrap();
        assert_eq!(
            summarize(&index),
            vec![((0, 2), 0, 2, 1), ((1, 2), 1, 2, 1)]
        );
    }

    /// REPL `(substring-index "5本")`: 3 entries; the counter slice
    /// `(0 2)` keeps 2 segments.
    #[tokio::test]
    async fn counter_5hon() {
        let ctx = ctx().await;
        let index = substring_index(&ctx, "5本").await.unwrap();
        assert_eq!(
            summarize(&index),
            vec![((0, 1), 0, 1, 1), ((0, 2), 0, 2, 2), ((1, 2), 1, 2, 2)]
        );
    }

    /// REPL `(substring-index "")`: empty input → empty index.
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

    // REPL-pinned (.103 SBCL 2.2.9, 2026-05-14):
    //   (ichiran/dict::gap-penalty 0 0)   => 0
    //   (ichiran/dict::gap-penalty 0 3)   => -1500
    //   (ichiran/dict::gap-penalty 7 9)   => -1000
    //   (ichiran/dict::gap-penalty 10 10) => 0
    //   (ichiran/dict::gap-penalty 5 2)   => 1500
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
            segments,
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
        // dict-grammar.lisp:1047-1054 — seg-left=nil + non-empty
        // satisfies-right → clause-2 pushes (nil, mslf(r, contradicts-right)).
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
            segments,
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

    // REPL probes (`/tmp/probe_gss_synth*.lisp` on .103, 2026-05-19).

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
    // Empty-input unit tests pinned against `.103` REPL probes
    // (SBCL 2.2.9, 2026-05-19). The DB-dependent non-empty paths
    // (outer loop, get-seg-initial, get-seg-splits accumulation) are
    // covered by the 522K-row audit binary at
    // `audit/dict/find_best_path_test.rs`.

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    // REPL: (ichiran/dict::find-best-path nil 5) => ((NIL . -2500))
    #[tokio::test]
    async fn empty_input_length_5_default_limit() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 5, None).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty(), "initial gap-seed has empty payload");
        assert_eq!(result[0].1, -2500);
    }

    // REPL: (ichiran/dict::find-best-path nil 0) => ((NIL . 0))
    #[tokio::test]
    async fn empty_input_length_0() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 0, None).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty());
        assert_eq!(result[0].1, 0);
    }

    // REPL: (ichiran/dict::find-best-path nil 1 :limit 3) => ((NIL . -500))
    #[tokio::test]
    async fn empty_input_length_1_limit_3() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 1, Some(3)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty());
        assert_eq!(result[0].1, -500);
    }

    // REPL: (ichiran/dict::find-best-path nil 1 :limit 1) => ((NIL . -500))
    #[tokio::test]
    async fn empty_input_length_1_limit_1() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 1, Some(1)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty());
        assert_eq!(result[0].1, -500);
    }
}
