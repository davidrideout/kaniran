mod get_original_text_once {
    use crate::dict::conj::make_conj_data;
    use crate::dict::readings::*;

    fn cd(pairs: &[(&str, &str)]) -> ConjData {
        make_conj_data(
            None,
            None,
            None,
            None,
            pairs
                .iter()
                .map(|(txt, src_txt)| (txt.to_string(), src_txt.to_string()))
                .collect(),
        )
    }

    /// Output order tracks src-map iteration order, not the input `texts`
    /// order — both two-text rows below collect `("たべる" "食べる")`
    /// regardless of how the texts are ordered.
    #[test]
    fn get_original_text_once_fixtures() {
        let cd1 = cd(&[
            ("たべます", "たべる"),
            ("喰べます", "喰べる"),
            ("食べます", "食べる"),
        ]);
        let cd2 = cd(&[
            ("たべない", "たべる"),
            ("喰べない", "喰べる"),
            ("食べない", "食べる"),
        ]);
        let cases: &[(&[ConjData], &[&str], &[&str])] = &[
            (std::slice::from_ref(&cd1), &["食べます"], &["食べる"]),
            (std::slice::from_ref(&cd1), &["たべます"], &["たべる"]),
            (
                std::slice::from_ref(&cd1),
                &["食べます", "たべます"],
                &["たべる", "食べる"],
            ),
            (
                std::slice::from_ref(&cd1),
                &["たべます", "食べます"],
                &["たべる", "食べる"],
            ),
            (std::slice::from_ref(&cd1), &["xyz"], &[]),
            (std::slice::from_ref(&cd1), &[], &[]),
            (
                std::slice::from_ref(&cd1),
                &["たべます", "喰べます", "食べます"],
                &["たべる", "喰べる", "食べる"],
            ),
            (
                &[cd1.clone(), cd2.clone()],
                &["食べます", "食べない"],
                &["食べる", "食べる"],
            ),
            (std::slice::from_ref(&cd2), &["食べない"], &["食べる"]),
            (&[], &["食べます"], &[]),
        ];
        for (conj_datas, texts, expected) in cases {
            let actual = get_original_text_once(conj_datas, texts);
            let actual_refs: Vec<&str> = actual.iter().map(String::as_str).collect();
            assert_eq!(actual_refs.as_slice(), *expected, "texts={texts:?}");
        }
    }
}

mod find_substring_words {
    use crate::dict::readings::*;
    // Each bucket is compared as a sorted `(seq, ord, common)` list: the
    // populating query has no ORDER BY, so the bucket order is not stable
    // and sorting both sides keeps the comparison order-independent. Run
    // with `cargo test -- --test-threads=1` per the DB-test convention.

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// A key's bucket as a sorted `(seq, ord, common)` list. Both this
    /// and the expected literal are in seq order so the unordered SQL
    /// bucket can't make the comparison flake.
    fn rows_sorted(h: &SubstringHash, key: &str) -> Vec<(i32, i32, Option<i32>)> {
        let mut out: Vec<(i32, i32, Option<i32>)> =
            match h.get(key).unwrap_or_else(|| panic!("missing key {key:?}")) {
                FindWordRows::Kana(v) => v.iter().map(|r| (r.seq, r.ord, r.common)).collect(),
                FindWordRows::Kanji(v) => v.iter().map(|r| (r.seq, r.ord, r.common)).collect(),
            };
        out.sort();
        out
    }

    fn keys_sorted(h: &SubstringHash) -> Vec<String> {
        let mut ks: Vec<String> = h.keys().cloned().collect();
        ks.sort();
        ks
    }

    fn is_kana(h: &SubstringHash, key: &str) -> bool {
        matches!(h.get(key), Some(FindWordRows::Kana(_)))
    }

    // 'こ' and 'ね' buckets are each shared by two tests — one copy here.
    fn ko_rows() -> Vec<(i32, i32, Option<i32>)> {
        vec![
            (1264740, 0, Some(0)),
            (1267110, 0, None),
            (1307770, 0, Some(1)),
            (1504770, 1, None),
            (1531190, 1, None),
            (1659920, 0, None),
            (1956240, 0, Some(28)),
            (2065150, 1, None),
            (2087990, 0, None),
            (2153770, 0, Some(0)),
            (2215030, 0, None),
            (2230390, 0, None),
            (2577750, 0, None),
            (2788170, 0, None),
            (2842951, 0, None),
            (2844354, 0, None),
        ]
    }

    fn ne_rows() -> Vec<(i32, i32, Option<i32>)> {
        vec![
            (1290020, 0, Some(5)),
            (1307780, 0, Some(0)),
            (1642760, 0, Some(15)),
            (2029080, 0, Some(0)),
            (2836242, 0, None),
            (2841117, 3, None),
            (2859162, 0, Some(0)),
            (10426293, 0, None),
        ]
    }

    #[tokio::test]
    async fn single_kanji_char_one_key() {
        // '猫' (no sticky): one kanji-classified key, two rows.
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "猫", &[]).await.unwrap();
        assert_eq!(keys_sorted(&h), vec!["猫".to_string()]);
        assert!(!is_kana(&h, "猫"), "'猫' should be kanji variant");
        assert_eq!(
            rows_sorted(&h, "猫"),
            vec![(1467640, 0, Some(7)), (2698030, 0, None)]
        );
    }

    #[tokio::test]
    async fn mixed_kana_kanji_three_keys() {
        // '猫が': が (7 kana), 猫 (2 kanji), 猫が (empty, kanji-classified
        // — the mixed string contains a kanji).
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "猫が", &[]).await.unwrap();
        assert_eq!(
            keys_sorted(&h),
            vec!["が".to_string(), "猫".to_string(), "猫が".to_string()]
        );
        assert!(is_kana(&h, "が"), "'が' should be kana variant");
        assert_eq!(
            rows_sorted(&h, "が"),
            vec![
                (1197760, 0, Some(40)),
                (1202270, 1, None),
                (2028930, 0, Some(0)),
                (2220800, 0, None),
                (2224630, 0, None),
                (2232110, 0, None),
                (2834041, 0, None),
            ]
        );
        assert!(!is_kana(&h, "猫"), "'猫' should be kanji variant");
        assert_eq!(
            rows_sorted(&h, "猫"),
            vec![(1467640, 0, Some(7)), (2698030, 0, None)]
        );
        assert!(!is_kana(&h, "猫が"), "mixed substring classified non-kana");
        assert!(rows_sorted(&h, "猫が").is_empty());
    }

    #[tokio::test]
    async fn sticky_end_blocks_substrings() {
        // '猫が' sticky=(1): every 1-char substring starts or ends at
        // pos 1, so only the length-2 key survives (empty bucket).
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "猫が", &[1]).await.unwrap();
        assert_eq!(keys_sorted(&h), vec!["猫が".to_string()]);
        assert!(rows_sorted(&h, "猫が").is_empty());
    }

    #[tokio::test]
    async fn sticky_start_and_end_block() {
        // 'ねこが' sticky=(0 3): start=0 and end=3 blocked, so only 'こ'
        // (start=1, end=2) survives.
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "ねこが", &[0, 3]).await.unwrap();
        assert_eq!(keys_sorted(&h), vec!["こ".to_string()]);
        assert_eq!(rows_sorted(&h, "こ"), ko_rows());
    }

    #[tokio::test]
    async fn sticky_interior_blocks_boundary_only() {
        // 'ねこが' sticky=(2): ね (8), こが (6), ねこが (empty). start=2
        // and end=2 are both blocked.
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "ねこが", &[2]).await.unwrap();
        assert_eq!(
            keys_sorted(&h),
            vec!["こが".to_string(), "ね".to_string(), "ねこが".to_string()]
        );
        assert_eq!(
            rows_sorted(&h, "こが"),
            vec![
                (1265180, 0, None),
                (1265190, 0, None),
                (10136364, 0, None),
                (10276500, 0, None),
                (12294787, 0, None),
                (12295833, 0, None),
            ]
        );
        assert_eq!(rows_sorted(&h, "ね"), ne_rows());
        assert!(rows_sorted(&h, "ねこが").is_empty());
    }

    #[tokio::test]
    async fn empty_string_empty_hash() {
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "", &[]).await.unwrap();
        assert!(h.is_empty());
    }

    #[tokio::test]
    async fn ascii_unknown_pre_seeds_empty_entry() {
        // 'x': one kanji-classified key, empty bucket — the pre-seeded
        // empty entry survives the no-row query.
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "x", &[]).await.unwrap();
        assert_eq!(keys_sorted(&h), vec!["x".to_string()]);
        assert!(
            !is_kana(&h, "x"),
            "'x' classified kanji (not in kana char set)"
        );
        assert!(rows_sorted(&h, "x").is_empty());
    }

    #[tokio::test]
    async fn full_kana_three_keys() {
        // 'ねこ': こ (16), ね (8), ねこ (1).
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "ねこ", &[]).await.unwrap();
        assert_eq!(
            keys_sorted(&h),
            vec!["こ".to_string(), "ね".to_string(), "ねこ".to_string()]
        );
        assert_eq!(rows_sorted(&h, "こ"), ko_rows());
        assert_eq!(rows_sorted(&h, "ね"), ne_rows());
        assert_eq!(rows_sorted(&h, "ねこ"), vec![(1467640, 0, Some(7))]);
    }

    /// A multi-row bucket must be the reverse of the database's fetch
    /// order. Compared unsorted, unlike the other tests here — that's the
    /// point. The expected order is derived from the same query the
    /// populator runs, so it pins the reversal rather than hard-coded seqs.
    #[tokio::test]
    async fn bucket_is_reverse_of_fetch_order() {
        let ctx = ctx_from_env().await;
        let keys = vec!["行って".to_string()];
        let fetch: Vec<i32> = sqlx::query_scalar("SELECT seq FROM kanji_text WHERE text = ANY($1)")
            .bind(&keys)
            .fetch_all(&ctx.pool)
            .await
            .unwrap();
        assert!(fetch.len() > 1, "test needs a multi-row bucket");
        let h = find_substring_words(&ctx, "行って", &[]).await.unwrap();
        let bucket: Vec<i32> = match h.get("行って").unwrap() {
            FindWordRows::Kanji(v) => v.iter().map(|r| r.seq).collect(),
            FindWordRows::Kana(v) => v.iter().map(|r| r.seq).collect(),
        };
        let mut expected = fetch;
        expected.reverse();
        assert_eq!(bucket, expected, "bucket must be reverse of fetch order");
    }
}

mod find_words_seqs {
    use crate::dict::readings::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    fn describe(word: &KaniWordDispatchEnum) -> (&'static str, i32, &str) {
        match word {
            KaniWordDispatchEnum::Kanji(k) => ("kanji", k.seq, k.text.as_str()),
            KaniWordDispatchEnum::Kana(k) => ("kana", k.seq, k.text.as_str()),
            _ => panic!("find_words_seqs must only return kanji-text / kana-text"),
        }
    }

    /// Each case returns one row: a kanji word lands in the kanji
    /// partition, a kana word in the kana partition.
    #[tokio::test]
    async fn single_row_fixtures() {
        let ctx = ctx().await;
        let cases: &[(&[&str], &[i32], (&str, i32, &str))] = &[
            (&["食べる"], &[1358280], ("kanji", 1358280, "食べる")),
            (&["たべる"], &[1358280], ("kana", 1358280, "たべる")),
            (&["見る"], &[1259290], ("kanji", 1259290, "見る")),
        ];
        for (words, seqs, expected) in cases {
            let result = find_words_seqs(&ctx, words, seqs).await.unwrap();
            assert_eq!(result.len(), 1, "words={words:?}");
            assert_eq!(describe(&result[0]), *expected, "words={words:?}");
        }
    }

    /// A kana word against six seqs returns one kana-text row per matching seq.
    #[tokio::test]
    async fn kana_multi_seq() {
        let ctx = ctx().await;
        let seqs = [1213770, 1259290, 1365450, 1772790, 2255060, 10553286];
        let result = find_words_seqs(&ctx, &["みる"], &seqs).await.unwrap();
        assert_eq!(result.len(), 6);
        let mut got: Vec<i32> = result
            .iter()
            .map(|word| {
                let (kind, seq, text) = describe(word);
                assert_eq!((kind, text), ("kana", "みる"));
                seq
            })
            .collect();
        got.sort_unstable();
        assert_eq!(got, seqs);
    }

    /// A kanji and a kana word return the kanji row first, then the kana
    /// row — both partitions non-empty.
    #[tokio::test]
    async fn mixed_two_words() {
        let ctx = ctx().await;
        let result = find_words_seqs(&ctx, &["食べる", "たべる"], &[1358280])
            .await
            .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(describe(&result[0]), ("kanji", 1358280, "食べる"));
        assert_eq!(describe(&result[1]), ("kana", 1358280, "たべる"));
    }

    /// All kanji rows precede all kana rows; order within each partition
    /// is the database's.
    #[tokio::test]
    async fn mixed_partition() {
        let ctx = ctx().await;
        let result = find_words_seqs(&ctx, &["見る", "みる", "食べる"], &[1259290, 1358280])
            .await
            .unwrap();
        assert_eq!(result.len(), 3);
        let kana_start = result
            .iter()
            .position(|word| matches!(word, KaniWordDispatchEnum::Kana(_)))
            .unwrap();
        assert!(result[..kana_start]
            .iter()
            .all(|word| matches!(word, KaniWordDispatchEnum::Kanji(_))));
        assert!(result[kana_start..]
            .iter()
            .all(|word| matches!(word, KaniWordDispatchEnum::Kana(_))));
        let mut got: Vec<(&str, i32, &str)> = result.iter().map(describe).collect();
        got.sort_unstable();
        let mut expected = vec![
            ("kanji", 1259290, "見る"),
            ("kanji", 1358280, "食べる"),
            ("kana", 1259290, "みる"),
        ];
        expected.sort_unstable();
        assert_eq!(got, expected);
    }

    /// The word matches a row but no row carries the requested seq, so the
    /// seq filter empties the result.
    #[tokio::test]
    async fn no_match_seq() {
        let ctx = ctx().await;
        let result = find_words_seqs(&ctx, &["食べる"], &[9999999])
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    /// Empty `words` skips every query and returns nothing.
    #[tokio::test]
    async fn empty_words() {
        let ctx = ctx().await;
        let result = find_words_seqs(&ctx, &[], &[1358280]).await.unwrap();
        assert!(result.is_empty());
    }
}

mod word_readings {
    use crate::dict::readings::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[tokio::test]
    async fn word_readings_fixtures() {
        let ctx = ctx_from_env().await;
        // (word, readings, romanizations). Cases cover:
        // - kana branch (word is itself in kana-text): word returned verbatim;
        // - kanji branch (ORDER BY id over the kana spellings): single & multi;
        // - katakana kana-branch input; macron long vowels; empty-on-both.
        let cases: &[(&str, &[&str], &[&str])] = &[
            // kanji branch, multiple kana readings ordered by id.
            (
                "猫",
                &["ねこ", "ネコ", "ねこま"],
                &["neko", "neko", "nekoma"],
            ),
            // kana branch — word is in kana-text, returned as-is.
            ("ねこ", &["ねこ"], &["neko"]),
            // kana branch, katakana surface form (long-vowel bar).
            ("コーヒー", &["コーヒー"], &["kohi"]),
            // kana branch, hiragana with macron long vowel.
            ("ありがとう", &["ありがとう"], &["arigatō"]),
            // kanji branch, single reading.
            ("図書館", &["としょかん"], &["toshokan"]),
            ("東京", &["とうきょう"], &["tōkyō"]),
            ("牛乳", &["ぎゅうにゅう"], &["gyūnyū"]),
            // mixed kanji+kana surface; not in kana-text, one kanji reading.
            ("食べる", &["たべる"], &["taberu"]),
            // not present in either table → empty kanji-seq → empty IN set.
            ("ヌルポポポ", &[], &[]),
        ];
        for (word, exp_readings, exp_rom) in cases {
            let (readings, romanizations) = word_readings(&ctx, word).await.unwrap();
            assert_eq!(&readings, exp_readings, "readings for {word}");
            assert_eq!(&romanizations, exp_rom, "romanizations for {word}");
        }
    }
}
