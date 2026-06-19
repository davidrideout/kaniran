mod parse_suffix_val {
    use crate::dict::grammar::suffix::resolve::*;
    use crate::dict::dao::SimpleText;

    fn kf(seq: i32, text: &str) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: text.to_string().into(),
            ord: 0,
            common: None,
            common_tags: String::new().into(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    /// REPL: `(parse-suffix-val "abc" nil)` → `NIL`.
    #[test]
    fn nil_val_yields_empty() {
        let out = parse_suffix_val("abc", None);
        assert!(out.is_empty());
    }

    /// REPL: `(parse-suffix-val "abc" '())` → `NIL`.
    /// Empty cache vec maps to empty output.
    #[test]
    fn empty_slice_yields_empty() {
        let v: Vec<(String, Option<KanaText>)> = Vec::new();
        let out = parse_suffix_val("abc", Some(&v));
        assert!(out.is_empty());
    }

    /// REPL: `(parse-suffix-val "ねえ" '(:nai nil))` →
    /// `(("ねえ" :NAI NIL))`.
    /// Mirrors the `load-abbr` cache shape (single entry with no kf).
    #[test]
    fn single_entry_no_kf() {
        let v = vec![("nai".to_string(), None)];
        let out = parse_suffix_val("ねえ", Some(&v));
        assert_eq!(out.len(), 1);
        let (substr, key, kana_form) = out[0];
        assert_eq!(substr, "ねえ");
        assert_eq!(key, "nai");
        assert!(kana_form.is_none());
    }

    /// REPL backing: matches the populator shape from `load_kf` —
    /// single 2-tuple entry `(key, Some(kf))` produces one triple.
    /// REPL parallel: `(parse-suffix-val "abc" '(:foo bar))` →
    /// `(("abc" :FOO BAR))` (Lisp flat-entry case; Rust always wraps
    /// in a 1-elem vec, same observable result).
    #[test]
    fn single_entry_with_kf() {
        let kana_form = kf(12345, "ちゃ");
        let v = vec![("chau".to_string(), Some(kana_form))];
        let out = parse_suffix_val("ちゃ", Some(&v));
        assert_eq!(out.len(), 1);
        let (substr, key, kf_ref) = out[0];
        assert_eq!(substr, "ちゃ");
        assert_eq!(key, "chau");
        assert_eq!(kf_ref.map(|k| k.seq), Some(12345));
    }

    /// REPL: `(parse-suffix-val "abc" '((:foo 1) (:bar 2)))` →
    /// `(("abc" :FOO 1) ("abc" :BAR 2))`.
    /// Order is preserved across the slice.
    #[test]
    fn multi_entry_preserves_order() {
        let v = vec![
            ("foo".to_string(), Some(kf(1, "x"))),
            ("bar".to_string(), Some(kf(2, "y"))),
        ];
        let out = parse_suffix_val("abc", Some(&v));
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0, "abc");
        assert_eq!(out[0].1, "foo");
        assert_eq!(out[0].2.map(|k| k.seq), Some(1));
        assert_eq!(out[1].0, "abc");
        assert_eq!(out[1].1, "bar");
        assert_eq!(out[1].2.map(|k| k.seq), Some(2));
    }
}

mod get_suffix_map {
    use crate::conn::kani_context::KaniranContext;
    use crate::dict::grammar::suffix::resolve::*;

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL: `(get-suffix-map "")` → empty hash-table. Length-0 input:
    /// the outer loop `from 0 below 0` is empty.
    #[test]
    fn empty_string_yields_empty() {
        let ctx = ctx_from_env();
        let result = get_suffix_map(&ctx, "");
        assert!(result.is_empty());
    }

    /// REPL: `(get-suffix-map "あ")` → empty hash-table. The single
    /// substring "あ" misses the cache.
    #[test]
    fn single_char_no_match_yields_empty() {
        let ctx = ctx_from_env();
        let result = get_suffix_map(&ctx, "あ");
        assert!(result.is_empty());
    }

    /// REPL: `(get-suffix-map "る")` →
    /// `end=1: (("る" :TEIRU #<KANA-TEXT 1577980 いる ord=0 common=0>))`.
    /// Length-1 input with a cache hit on the whole string.
    #[test]
    fn single_char_match_at_end_one() {
        let ctx = ctx_from_env();
        let result = get_suffix_map(&ctx, "る");
        assert_eq!(result.len(), 1);
        let items = &result[&1];
        assert_eq!(items.len(), 1);
        let (substr, key, kf) = items[0];
        assert_eq!(substr, "る");
        assert_eq!(key, "teiru");
        let kf = kf.expect("kf row present");
        assert_eq!(kf.seq, 1577980);
        assert_eq!(kf.text, "いる");
        assert_eq!(kf.ord, 0);
        assert_eq!(kf.common, Some(0));
    }

    /// REPL: `(get-suffix-map "食べてる")` →
    /// `end=3: (("て" :TEIRU #<KANA-TEXT 10551841 いて ord=0 common=NULL>))`
    /// `end=4: (("る" :TEIRU #<KANA-TEXT 1577980 いる ord=0 common=0>))`.
    /// Two distinct ends, one match each, across multi-byte input.
    #[test]
    fn taberu_two_distinct_ends() {
        let ctx = ctx_from_env();
        let result = get_suffix_map(&ctx, "食べてる");
        assert_eq!(result.len(), 2);

        let end3 = &result[&3];
        assert_eq!(end3.len(), 1);
        let (substr, key, kf) = end3[0];
        assert_eq!(substr, "て");
        assert_eq!(key, "teiru");
        let kf = kf.expect("kf row present");
        crate::test_support::check_base_seqs(kf.seq, &[1577980]);
        assert_eq!(kf.text, "いて");
        assert_eq!(kf.ord, 0);
        assert_eq!(kf.common, None);

        let end4 = &result[&4];
        assert_eq!(end4.len(), 1);
        let (substr, key, kf) = end4[0];
        assert_eq!(substr, "る");
        assert_eq!(key, "teiru");
        let kf = kf.expect("kf row present");
        assert_eq!(kf.seq, 1577980);
        assert_eq!(kf.text, "いる");
        assert_eq!(kf.ord, 0);
        assert_eq!(kf.common, Some(0));
    }

    /// REPL: `(get-suffix-map "飲みたい")` →
    /// `end=3: (("た" :TEIRU #<KANA-TEXT 10551837 いた ord=0 common=NULL>))`
    /// `end=4: (("い" :TEIRU #<KANA-TEXT 2258170 い ord=0 common=NULL>)
    ///          ("たい" :TAI #<KANA-TEXT 2017560 たい ord=0 common=0>))`.
    /// end=4 holds two matches (substrings "い" at start=3 and "たい" at
    /// start=2); the prepend order places start=3's match first.
    #[test]
    fn nomitai_multiple_at_same_end() {
        let ctx = ctx_from_env();
        let result = get_suffix_map(&ctx, "飲みたい");
        assert_eq!(result.len(), 2);

        let end3 = &result[&3];
        assert_eq!(end3.len(), 1);
        let (substr, key, kf) = end3[0];
        assert_eq!(substr, "た");
        assert_eq!(key, "teiru");
        let kf = kf.expect("kf row present");
        crate::test_support::check_base_seqs(kf.seq, &[1577980]);
        assert_eq!(kf.text, "いた");
        assert_eq!(kf.ord, 0);
        assert_eq!(kf.common, None);

        let end4 = &result[&4];
        assert_eq!(end4.len(), 2);
        let (substr0, key0, kf0) = end4[0];
        assert_eq!(substr0, "い");
        assert_eq!(key0, "teiru");
        let kf0 = kf0.expect("kf0 row present");
        assert_eq!(kf0.seq, 2258170);
        assert_eq!(kf0.text, "い");
        assert_eq!(kf0.ord, 0);
        assert_eq!(kf0.common, None);
        let (substr1, key1, kf1) = end4[1];
        assert_eq!(substr1, "たい");
        assert_eq!(key1, "tai");
        let kf1 = kf1.expect("kf1 row present");
        assert_eq!(kf1.seq, 2017560);
        assert_eq!(kf1.text, "たい");
        assert_eq!(kf1.ord, 0);
        assert_eq!(kf1.common, Some(0));
    }

    /// REPL: `(get-suffix-map "見たくない")` →
    /// `end=2: (("た" :TEIRU #<KANA-TEXT 10551837 いた ord=0 common=NULL>))`
    /// `end=3: (("く" :TE #<KANA-TEXT 1578850 いく ord=0 common=0>)
    ///          ("たく" :TAI #<KANA-TEXT 10477471 たく ord=0 common=NULL>))`
    /// `end=5: (("い" :TEIRU #<KANA-TEXT 2258170 い ord=0 common=NULL>)
    ///          ("ない" :TEIRU #<KANA-TEXT 10551835 いない ord=0 common=NULL>)
    ///          ("たくない" :TAI #<KANA-TEXT 10477455 たくない ord=0 common=NULL>))`.
    /// end=5 holds three matches at starts 4/3/1; the prepend order is
    /// shortest-substring-first (start=4 → start=1).
    #[test]
    fn mitakunai_prepend_order_across_starts() {
        let ctx = ctx_from_env();
        let result = get_suffix_map(&ctx, "見たくない");
        assert_eq!(result.len(), 3);

        let end2 = &result[&2];
        assert_eq!(end2.len(), 1);
        let (substr, key, kf) = end2[0];
        assert_eq!(substr, "た");
        assert_eq!(key, "teiru");
        let kf = kf.expect("kf row present");
        crate::test_support::check_base_seqs(kf.seq, &[1577980]);
        assert_eq!(kf.text, "いた");

        let end3 = &result[&3];
        assert_eq!(end3.len(), 2);
        let (substr0, key0, kf0) = end3[0];
        assert_eq!(substr0, "く");
        assert_eq!(key0, "te");
        let kf0 = kf0.expect("kf0 row present");
        assert_eq!(kf0.seq, 1578850);
        assert_eq!(kf0.text, "いく");
        assert_eq!(kf0.common, Some(0));
        let (substr1, key1, kf1) = end3[1];
        assert_eq!(substr1, "たく");
        assert_eq!(key1, "tai");
        let kf1 = kf1.expect("kf1 row present");
        crate::test_support::check_base_seqs(kf1.seq, &[2017560]);
        assert_eq!(kf1.text, "たく");
        assert_eq!(kf1.common, None);

        let end5 = &result[&5];
        assert_eq!(end5.len(), 3);
        let (substr0, key0, kf0) = end5[0];
        assert_eq!(substr0, "い");
        assert_eq!(key0, "teiru");
        assert_eq!(kf0.expect("kf0 row present").seq, 2258170);
        let (substr1, key1, kf1) = end5[1];
        assert_eq!(substr1, "ない");
        assert_eq!(key1, "teiru");
        let kf1 = kf1.expect("kf1 row present");
        crate::test_support::check_base_seqs(kf1.seq, &[1577980]);
        assert_eq!(kf1.text, "いない");
        let (substr2, key2, kf2) = end5[2];
        assert_eq!(substr2, "たくない");
        assert_eq!(key2, "tai");
        let kf2 = kf2.expect("kf2 row present");
        crate::test_support::check_base_seqs(kf2.seq, &[2017560]);
        assert_eq!(kf2.text, "たくない");
    }

    /// REPL: `(get-suffix-map "本を読んでいる")` →
    /// `end=4: (("ん" :NAI-N NIL))`
    /// `end=6: (("い" :TEIRU #<KANA-TEXT 2258170 い ord=0 common=NULL>))`
    /// `end=7: (("る" :TEIRU #<KANA-TEXT 1577980 いる ord=0 common=0>)
    ///          ("いる" :TEIRU+ #<KANA-TEXT 1577980 いる ord=0 common=0>))`.
    /// Exercises the `kf = None` case (`:NAI-N`) and the `:TEIRU+`
    /// class, plus two matches sharing end=7.
    #[test]
    fn yondeiru_none_kf_and_teiru_plus() {
        let ctx = ctx_from_env();
        let result = get_suffix_map(&ctx, "本を読んでいる");
        assert_eq!(result.len(), 3);

        let end4 = &result[&4];
        assert_eq!(end4.len(), 1);
        let (substr, key, kf) = end4[0];
        assert_eq!(substr, "ん");
        assert_eq!(key, "nai-n");
        assert!(kf.is_none());

        let end6 = &result[&6];
        assert_eq!(end6.len(), 1);
        let (substr, key, kf) = end6[0];
        assert_eq!(substr, "い");
        assert_eq!(key, "teiru");
        assert_eq!(kf.expect("kf row present").seq, 2258170);

        let end7 = &result[&7];
        assert_eq!(end7.len(), 2);
        let (substr0, key0, kf0) = end7[0];
        assert_eq!(substr0, "る");
        assert_eq!(key0, "teiru");
        let kf0 = kf0.expect("kf0 row present");
        assert_eq!(kf0.seq, 1577980);
        assert_eq!(kf0.text, "いる");
        assert_eq!(kf0.common, Some(0));
        let (substr1, key1, kf1) = end7[1];
        assert_eq!(substr1, "いる");
        assert_eq!(key1, "teiru+");
        let kf1 = kf1.expect("kf1 row present");
        assert_eq!(kf1.seq, 1577980);
        assert_eq!(kf1.text, "いる");
        assert_eq!(kf1.common, Some(0));
    }
}

mod get_suffixes {
    use crate::conn::kani_context::KaniranContext;
    use crate::dict::grammar::suffix::resolve::*;

    fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL: `(get-suffixes "")` → `NIL`. Length-0 word: loop range
    /// `from -1 downto 1` is empty.
    #[test]
    fn empty_word_yields_empty() {
        let ctx = ctx_from_env();
        let out = get_suffixes(&ctx, "");
        assert!(out.is_empty());
    }

    /// REPL: `(get-suffixes "あ")` → `NIL`. Length-1 word: loop range
    /// `from 0 downto 1` is empty.
    #[test]
    fn single_char_word_yields_empty() {
        let ctx = ctx_from_env();
        let out = get_suffixes(&ctx, "あ");
        assert!(out.is_empty());
    }

    /// REPL: `(get-suffixes "る")` → `NIL`. Even when the trailing char
    /// has a cache entry, length-1 input never enters the loop body.
    #[test]
    fn cached_substr_at_length_one_yields_empty() {
        let ctx = ctx_from_env();
        let out = get_suffixes(&ctx, "る");
        assert!(out.is_empty());
    }

    /// REPL: `(get-suffixes "abcde")` → `NIL`. ASCII word with no
    /// cached substrings — exercises the loop walking 4 starts with
    /// only `gethash` misses.
    #[test]
    fn no_match_yields_empty() {
        let ctx = ctx_from_env();
        let out = get_suffixes(&ctx, "abcde");
        assert!(out.is_empty());
    }

    /// REPL: `(get-suffixes "食べてる")` →
    /// `(("る" :TEIRU #<KANA-TEXT 1577980 いる ord=0 common=0>))`.
    /// Single suffix match at the deepest start (start=3 → "る").
    #[test]
    fn single_match_at_deepest_start() {
        let ctx = ctx_from_env();
        let out = get_suffixes(&ctx, "食べてる");
        assert_eq!(out.len(), 1);
        let (substr, key, kf) = out[0];
        assert_eq!(substr, "る");
        assert_eq!(key, "teiru");
        let kf = kf.expect("kf row present");
        assert_eq!(kf.seq, 1577980);
        assert_eq!(kf.text, "いる");
        assert_eq!(kf.ord, 0);
        assert_eq!(kf.common, Some(0));
    }

    /// REPL: `(get-suffixes "飲みたい")` →
    /// `(("い" :TEIRU #<KANA-TEXT 2258170 い ord=0 common=NULL>)
    ///   ("たい" :TAI #<KANA-TEXT 2017560 たい ord=0 common=0>))`.
    /// Two matches at different starts; order is start=3 then start=2.
    #[test]
    fn multiple_matches_in_decreasing_start_order() {
        let ctx = ctx_from_env();
        let out = get_suffixes(&ctx, "飲みたい");
        assert_eq!(out.len(), 2);
        let (s0, k0, kf0) = out[0];
        let (s1, k1, kf1) = out[1];
        assert_eq!(s0, "い");
        assert_eq!(k0, "teiru");
        let kf0 = kf0.expect("kf0 row present");
        assert_eq!(kf0.seq, 2258170);
        assert_eq!(kf0.text, "い");
        assert_eq!(kf0.ord, 0);
        assert_eq!(kf0.common, None);
        assert_eq!(s1, "たい");
        assert_eq!(k1, "tai");
        let kf1 = kf1.expect("kf1 row present");
        assert_eq!(kf1.seq, 2017560);
        assert_eq!(kf1.text, "たい");
        assert_eq!(kf1.ord, 0);
        assert_eq!(kf1.common, Some(0));
    }

    /// REPL: `(get-suffixes "食べてました")` →
    /// `(("た" :TEIRU #<KANA-TEXT 10551837 いた ord=0 common=NULL>)
    ///   ("した" :SURU #<KANA-TEXT 10152246 した ord=0 common=NULL>)
    ///   ("ました" :TEIRU #<KANA-TEXT 10551838 いました ord=0 common=NULL>))`.
    /// Three matches — pins the shortest-suffix-first ordering across
    /// a 6-char input where the loop visits start=5,4,3,2,1.
    #[test]
    fn three_matches_shortest_suffix_first() {
        let ctx = ctx_from_env();
        let out = get_suffixes(&ctx, "食べてました");
        assert_eq!(out.len(), 3);
        let (s0, k0, kf0) = out[0];
        let (s1, k1, kf1) = out[1];
        let (s2, k2, kf2) = out[2];
        assert_eq!(s0, "た");
        assert_eq!(k0, "teiru");
        let kf0 = kf0.expect("kf0 row present");
        crate::test_support::check_base_seqs(kf0.seq, &[1577980]);
        assert_eq!(kf0.text, "いた");
        assert_eq!(kf0.ord, 0);
        assert_eq!(kf0.common, None);
        assert_eq!(s1, "した");
        assert_eq!(k1, "suru");
        let kf1 = kf1.expect("kf1 row present");
        crate::test_support::check_base_seqs(kf1.seq, &[1157170]);
        assert_eq!(kf1.text, "した");
        assert_eq!(kf1.ord, 0);
        assert_eq!(kf1.common, None);
        assert_eq!(s2, "ました");
        assert_eq!(k2, "teiru");
        let kf2 = kf2.expect("kf2 row present");
        crate::test_support::check_base_seqs(kf2.seq, &[1577980]);
        assert_eq!(kf2.text, "いました");
        assert_eq!(kf2.ord, 0);
        assert_eq!(kf2.common, None);
    }
}

mod match_unique {
    use crate::conn::kani_context::KaniranContext;
    use crate::dict::grammar::suffix::resolve::*;
    use crate::dict::dao::KanaText;
    use crate::dict::dao::SimpleText;

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    #[cfg(feature = "postgres")]
    fn fetch_kana_rows_for_seq(ctx: &KaniranContext, seq_val: i32) -> Vec<KanaText> {
        tokio::runtime::Runtime::new()
            .expect("tokio runtime")
            .block_on(
                sqlx::query_as::<_, KanaText>("SELECT * FROM kana_text WHERE seq = $1")
                    .bind(seq_val)
                    .fetch_all(ctx.pool.as_ref().expect("postgres pool")),
            )
            .expect("query kana_text")
    }

    fn wrap(rows: Vec<KanaText>) -> Vec<KaniWordDispatchEnum> {
        rows.into_iter().map(KaniWordDispatchEnum::Kana).collect()
    }

    fn synthetic_kana(seq_val: i32) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: seq_val,
            text: String::new().into(),
            ord: 0,
            common: None,
            common_tags: String::new().into(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    // REPL: (match-unique :ii nil) => :II
    #[test]
    fn bare_ii_with_empty_matches_returns_bare() {
        let c = ctx();
        let out = match_unique(&c, "ii", &[]).unwrap();
        assert_eq!(out, Some(MatchUniqueResult::Bare));
    }

    // REPL: (match-unique :ra nil) => :RA
    #[test]
    fn bare_ra_returns_bare() {
        let c = ctx();
        let out = match_unique(&c, "ra", &[]).unwrap();
        assert_eq!(out, Some(MatchUniqueResult::Bare));
    }

    // REPL: (match-unique :mo nil) => :MO
    #[test]
    fn bare_mo_returns_bare() {
        let c = ctx();
        let out = match_unique(&c, "mo", &[]).unwrap();
        assert_eq!(out, Some(MatchUniqueResult::Bare));
    }

    // REPL: (match-unique :unknown nil) => NIL
    // REPL: (match-unique :foo nil) => NIL
    #[test]
    fn unknown_class_returns_none() {
        let c = ctx();
        assert_eq!(match_unique(&c, "unknown", &[]).unwrap(), None);
        assert_eq!(match_unique(&c, "foo", &[]).unwrap(), None);
    }

    // REPL: (match-unique :sa nil) => NIL
    #[test]
    fn sa_with_empty_matches_returns_none() {
        let c = ctx();
        let out = match_unique(&c, "sa", &[]).unwrap();
        assert_eq!(out, None);
    }

    // REPL: matches = kana-text rows for seq=10243330 only (non-root).
    //       (match-unique :sa matches) => NIL
    #[cfg(feature = "postgres")]
    #[test]
    fn sa_with_only_non_root_seqs_returns_none() {
        let c = ctx();
        let mats = wrap(fetch_kana_rows_for_seq(&c, 10243330));
        assert!(
            !mats.is_empty(),
            "REPL precondition: kana_text rows exist for seq=10243330"
        );
        let out = match_unique(&c, "sa", &mats).unwrap();
        assert_eq!(out, None);
    }

    // REPL: matches = kana-text rows for seq=10243330 + seq=1586010
    //       (the latter is root-p). (match-unique :sa matches) => (1586010)
    #[cfg(feature = "postgres")]
    #[test]
    fn sa_with_mixed_root_returns_root_seqs() {
        let c = ctx();
        let mut mats = fetch_kana_rows_for_seq(&c, 10243330);
        mats.extend(fetch_kana_rows_for_seq(&c, 1586010));
        let wrapped = wrap(mats);
        let out = match_unique(&c, "sa", &wrapped).unwrap();
        assert_eq!(out, Some(MatchUniqueResult::Sa(vec![1586010])));
    }

    // REPL: (match-unique :sa (find-word "はや"))
    //   matches seqs: 1586010 1956580 2638250 10243330
    //     => (1586010 1956580 2638250)
    #[cfg(feature = "postgres")]
    #[test]
    fn sa_with_haya_matches_returns_three_root_seqs() {
        let c = ctx();
        let mut mats = Vec::new();
        for s in [1586010, 1956580, 2638250, 10243330] {
            mats.extend(fetch_kana_rows_for_seq(&c, s));
        }
        let wrapped = wrap(mats);
        let out = match_unique(&c, "sa", &wrapped).unwrap();
        let Some(MatchUniqueResult::Sa(mut rows)) = out else {
            panic!("expected Some(Sa(..)), got {:?}", out);
        };
        rows.sort();
        assert_eq!(rows, vec![1586010, 1956580, 2638250]);
    }

    // REPL: (match-unique :desu nil) => NIL
    #[test]
    fn desu_with_empty_matches_returns_none() {
        let c = ctx();
        let out = match_unique(&c, "desu", &[]).unwrap();
        assert_eq!(out, None);
    }

    // REPL: matches = 2 kana_text rows for seq=10597478 (じゃないです variants)
    //       seqs unique → 1; conj rows from 2755350 → 1; (< 1 2) = T
    //       (match-unique :desu matches) => T
    #[cfg(feature = "postgres")]
    #[test]
    fn desu_with_duplicate_jyanai_seqs_returns_desu() {
        let c = ctx();
        let mats = wrap(fetch_kana_rows_for_seq(&c, 10597478));
        assert_eq!(
            mats.len(),
            2,
            "REPL precondition: kana_text rows for seq=10597478 = 2"
        );
        let out = match_unique(&c, "desu", &mats).unwrap();
        assert_eq!(out, Some(MatchUniqueResult::Desu));
    }

    // REPL: matches = all kana_text rows for seqs 10597478, 10597479, 10597480
    //       len=8; conj rows from 2755350 (3 unique seqs) = 3; (< 3 8) = T
    //       (match-unique :desu matches) => T
    #[cfg(feature = "postgres")]
    #[test]
    fn desu_with_all_jyanai_derived_returns_desu() {
        let c = ctx();
        let mut mats = Vec::new();
        for s in [10597478, 10597479, 10597480] {
            mats.extend(fetch_kana_rows_for_seq(&c, s));
        }
        let wrapped = wrap(mats);
        assert_eq!(
            wrapped.len(),
            8,
            "REPL precondition: 8 kana_text rows across the three seqs"
        );
        let out = match_unique(&c, "desu", &wrapped).unwrap();
        assert_eq!(out, Some(MatchUniqueResult::Desu));
    }

    // REPL: matches = 2 rows for seq=10597478 + 3 rows for seq=1586010 (not じゃない-derived)
    //       conj rows for {10597478, 1586010} from 2755350 = 1; (< 1 5) = T
    //       (match-unique :desu mixed) => T
    #[cfg(feature = "postgres")]
    #[test]
    fn desu_with_mixed_jyanai_and_other_returns_desu() {
        let c = ctx();
        let mut mats = fetch_kana_rows_for_seq(&c, 10597478);
        mats.extend(fetch_kana_rows_for_seq(&c, 1586010));
        let wrapped = wrap(mats);
        assert_eq!(wrapped.len(), 5);
        let out = match_unique(&c, "desu", &wrapped).unwrap();
        assert_eq!(out, Some(MatchUniqueResult::Desu));
    }

    // REPL: (make-instance 'compound-text … :words (list kt1 kt2)) → (seq …)
    //       returns the children's seqs as a list; (match-unique :sa (list ct))
    //       errors with Postgres "42883: operator does not exist: integer = record".
    //       The Rust port mirrors this failure mode by panicking — pinning the
    //       behavior so a future caller change doesn't silently substitute
    //       a different shape.
    #[test]
    #[should_panic(expected = "compound-text seq returned WordInfoSeq::Multi")]
    fn sa_with_compound_text_match_panics() {
        use crate::dict::text_classes::{CompoundText, ScoreMod};
        let c = ctx();
        let child1 = synthetic_kana(1586010);
        let child2 = synthetic_kana(10597478);
        let compound = KaniWordDispatchEnum::Compound(CompoundText {
            text: "compound".into(),
            kana: "compound".into(),
            primary: Box::new(child1.clone()),
            words: vec![child1, child2],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        });
        let _ = match_unique(&c, "sa", &[compound]);
    }

    // Same panic must fire for the :desu DB branch — both querying paths
    // share `collect_seqs` and must abort identically.
    #[test]
    #[should_panic(expected = "compound-text seq returned WordInfoSeq::Multi")]
    fn desu_with_compound_text_match_panics() {
        use crate::dict::text_classes::{CompoundText, ScoreMod};
        let c = ctx();
        let child1 = synthetic_kana(1586010);
        let child2 = synthetic_kana(10597478);
        let compound = KaniWordDispatchEnum::Compound(CompoundText {
            text: "compound".into(),
            kana: "compound".into(),
            primary: Box::new(child1.clone()),
            words: vec![child1, child2],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        });
        let _ = match_unique(&c, "desu", &[compound]);
    }
}

mod find_word_suffix {
    use crate::dict::grammar::suffix::resolve::*;

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// REPL: `(find-word-suffix "勉強する")` upstream returns 1
    /// compound via the SURU branch (TEIRU also reaches "る" but
    /// suffix-teiru on root="勉強す" fails its te-check).
    #[test]
    fn t1_benkyou_suru() {
        let ctx = ctx();
        let r = find_word_suffix(&ctx, "勉強する", &[]).unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected Compound, got {:?}", r[0]);
        };
        assert_eq!(c.text, "勉強する");
        assert_eq!(c.kana, "べんきょう する");
    }

    /// REPL: `(find-word-suffix "区別し")` → 1 compound (SURU branch
    /// only — the partial cache holds an entry for "し" under :SURU
    /// keyword).
    #[test]
    fn t2_kubetsu_shi() {
        let ctx = ctx();
        let r = find_word_suffix(&ctx, "区別し", &[]).unwrap();
        assert_eq!(r.len(), 1);
    }

    /// REPL: `(find-word-suffix "私ら")` → 13 compounds via the RA
    /// branch.
    #[test]
    fn t3_watashi_ra_polysemy() {
        let ctx = ctx();
        let r = find_word_suffix(&ctx, "私ら", &[]).unwrap();
        assert_eq!(r.len(), 13);
        for w in &r {
            let KaniWordDispatchEnum::Compound(c) = w else {
                panic!("expected Compound, got {:?}", w);
            };
            assert_eq!(c.text, "私ら");
        }
    }

    /// REPL: `(find-word-suffix "食べてる")` upstream returns 1 via
    /// the TEIRU branch (suffix-teiru's te-check passes on root
    /// "食べて"). The dispatch table now wires `teiru`, so we mirror
    /// upstream and pin the 1-compound outcome.
    #[test]
    fn t4_teiru_fires() {
        let ctx = ctx();
        let r = find_word_suffix(&ctx, "食べてる", &[]).unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected Compound, got {:?}", r[0]);
        };
        assert_eq!(c.text, "食べてる");
    }

    /// REPL: `(find-word-suffix "ら")` → NIL. Word length equals
    /// suffix length → offset = 0 → `(> offset 0)` fails → no
    /// expansion.
    #[test]
    fn t5_offset_zero_skipped() {
        let ctx = ctx();
        let r = find_word_suffix(&ctx, "ら", &[]).unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-suffix "")` → NIL. get-suffixes("") = NIL
    /// (loop range empty), so the iteration body doesn't run.
    #[test]
    fn t6_empty_word() {
        let ctx = ctx();
        let r = find_word_suffix(&ctx, "", &[]).unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-suffix "私ら" :matches (find-word "私"))` →
    /// NIL. `match-unique` :ra returns :RA (truthy) for the find-word
    /// 私 matches → the row is filtered out and no compounds emit.
    #[test]
    fn t7_match_unique_gate_fires() {
        let ctx = ctx();
        // Build matches = find-word 私 (kana + kanji rows).
        let watashi_rows = crate::dict::readings::find_word(&ctx, "私", false)
            
            .unwrap()
            .into_owned();
        let matches: Vec<KaniWordDispatchEnum> = match watashi_rows {
            crate::dict::readings::FindWordRows::Kana(v) => {
                v.into_iter().map(KaniWordDispatchEnum::Kana).collect()
            }
            crate::dict::readings::FindWordRows::Kanji(v) => {
                v.into_iter().map(KaniWordDispatchEnum::Kanji).collect()
            }
        };
        assert!(!matches.is_empty(), "REPL precondition: 私 rows exist");
        let r = find_word_suffix(&ctx, "私ら", &matches).unwrap();
        assert!(r.is_empty());
    }

    /// Map-path branch coverage (`dict-grammar.lisp:697` — the
    /// `*suffix-map-temp*` source, `find_word_suffix.rs:95-103`). Every
    /// other test here runs with `suffix_map_temp = None` and exercises
    /// only the `get_suffixes` fallback; this one binds a real suffix
    /// map (mirroring `join_substring_words_star_`) so the suffix triples
    /// come from `map[suffix_next_end]`, independent of `word`.
    ///
    /// Sentence "しきれなくなったらしく" — なくなったら ends at char 9.
    /// REPL-verified on the ichiran host: map@9 = (ら たら ったら なったら)
    /// → `find-word-suffix("なくなったら")` = 3; map@8 = (た った なった)
    /// → 0. The next-end=8 case is the nested-call shape (a parent suffix
    /// decremented the end): the map is indexed one position short,
    /// yields the wrong suffix row, and returns 0 where the bare
    /// `get_suffixes` path would have returned 3.
    #[test]
    fn t8_map_path_position_sensitive() {
        use crate::dict::word_info::SuffixMapTemp;
        use crate::dict::grammar::suffix::resolve::get_suffix_map;
        use std::sync::Arc;

        let ctx = ctx();
        let sentence = "しきれなくなったらしく";
        // Mirror join_substring_words_star_:72-83 — *suffix-map-temp*
        // owns its triples, so materialize owned copies of the borrowed
        // get_suffix_map output.
        let suffix_map: Arc<SuffixMapTemp> = Arc::new(
            get_suffix_map(&ctx, sentence)
                .into_iter()
                .map(|(end, items)| {
                    let owned: Vec<(String, String, Option<_>)> = items
                        .into_iter()
                        .map(|(s, k, kf)| (s.to_string(), k.to_string(), kf.cloned()))
                        .collect();
                    (end, owned)
                })
                .collect(),
        );

        // map@9 = (ら たら ったら なったら) → 3 compounds.
        let ctx9 = ctx
            .with_suffix_map_temp(Some(Arc::clone(&suffix_map)))
            .with_suffix_next_end(Some(9));
        let r9 = find_word_suffix(&ctx9, "なくなったら", &[]).unwrap();
        assert_eq!(r9.len(), 3, "map@9 (ら/たら/ったら/なったら) → 3 compounds");

        // map@8 = (た った なった) — the decremented-end nested-call
        // shape; the suffixes don't align with なくなったら → 0.
        let ctx8 = ctx
            .with_suffix_map_temp(Some(Arc::clone(&suffix_map)))
            .with_suffix_next_end(Some(8));
        let r8 = find_word_suffix(&ctx8, "なくなったら", &[]).unwrap();
        assert!(r8.is_empty(), "map@8 (た/った/なった) → no compounds");
    }
}
