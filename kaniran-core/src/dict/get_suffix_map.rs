//! Port of `ichiran/dict:get-suffix-map` (`dict-grammar.lisp:671`).
//!
//! For every substring of `str`, looks up the suffix cache and maps
//! each parsed suffix item under the substring's end position. Loop
//! bounds and `subseq-slice` are character offsets, not byte offsets.

use crate::conn::kani_context::KaniranContext;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::parse_suffix_val::parse_suffix_val;
use crate::dict::subseq_slice::subseq_slice;
use std::collections::HashMap;

pub fn get_suffix_map<'a, 'b>(
    ctx: &'a KaniranContext,
    str: &'b str,
) -> HashMap<usize, Vec<(&'b str, &'a str, Option<&'a KanaText>)>> {
    // dict-grammar.lisp:672 (init-suffixes) — eager-population invariant
    // of KaniranContext::from_url.
    assert!(
        !ctx.suffix_cache.is_empty(),
        "get-suffix-map: ctx.suffix_cache is empty (init-suffixes contract violated)",
    );
    let mut result: HashMap<usize, Vec<(&'b str, &'a str, Option<&'a KanaText>)>> = HashMap::new();
    let length = str.chars().count();
    for start in 0..length {
        for end in (start + 1)..=length {
            let substr = subseq_slice(None, str, start, Some(end));
            let val = ctx.suffix_cache.get(substr).map(|v| v.as_slice());
            for item in parse_suffix_val(substr, val) {
                // dict-grammar.lisp:679 (push item (gethash end result nil))
                result.entry(end).or_default().insert(0, item);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conn::kani_context::KaniranContext;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL: `(get-suffix-map "")` → empty hash-table. Length-0 input:
    /// the outer loop `from 0 below 0` is empty.
    #[tokio::test]
    async fn empty_string_yields_empty() {
        let ctx = ctx_from_env().await;
        let result = get_suffix_map(&ctx, "");
        assert!(result.is_empty());
    }

    /// REPL: `(get-suffix-map "あ")` → empty hash-table. The single
    /// substring "あ" misses the cache.
    #[tokio::test]
    async fn single_char_no_match_yields_empty() {
        let ctx = ctx_from_env().await;
        let result = get_suffix_map(&ctx, "あ");
        assert!(result.is_empty());
    }

    /// REPL: `(get-suffix-map "る")` →
    /// `end=1: (("る" :TEIRU #<KANA-TEXT 1577980 いる ord=0 common=0>))`.
    /// Length-1 input with a cache hit on the whole string.
    #[tokio::test]
    async fn single_char_match_at_end_one() {
        let ctx = ctx_from_env().await;
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
    #[tokio::test]
    async fn taberu_two_distinct_ends() {
        let ctx = ctx_from_env().await;
        let result = get_suffix_map(&ctx, "食べてる");
        assert_eq!(result.len(), 2);

        let end3 = &result[&3];
        assert_eq!(end3.len(), 1);
        let (substr, key, kf) = end3[0];
        assert_eq!(substr, "て");
        assert_eq!(key, "teiru");
        let kf = kf.expect("kf row present");
        assert_eq!(kf.seq, 10551841);
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
    #[tokio::test]
    async fn nomitai_multiple_at_same_end() {
        let ctx = ctx_from_env().await;
        let result = get_suffix_map(&ctx, "飲みたい");
        assert_eq!(result.len(), 2);

        let end3 = &result[&3];
        assert_eq!(end3.len(), 1);
        let (substr, key, kf) = end3[0];
        assert_eq!(substr, "た");
        assert_eq!(key, "teiru");
        let kf = kf.expect("kf row present");
        assert_eq!(kf.seq, 10551837);
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
    #[tokio::test]
    async fn mitakunai_prepend_order_across_starts() {
        let ctx = ctx_from_env().await;
        let result = get_suffix_map(&ctx, "見たくない");
        assert_eq!(result.len(), 3);

        let end2 = &result[&2];
        assert_eq!(end2.len(), 1);
        let (substr, key, kf) = end2[0];
        assert_eq!(substr, "た");
        assert_eq!(key, "teiru");
        let kf = kf.expect("kf row present");
        assert_eq!(kf.seq, 10551837);
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
        assert_eq!(kf1.seq, 10477471);
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
        assert_eq!(kf1.seq, 10551835);
        assert_eq!(kf1.text, "いない");
        let (substr2, key2, kf2) = end5[2];
        assert_eq!(substr2, "たくない");
        assert_eq!(key2, "tai");
        let kf2 = kf2.expect("kf2 row present");
        assert_eq!(kf2.seq, 10477455);
        assert_eq!(kf2.text, "たくない");
    }

    /// REPL: `(get-suffix-map "本を読んでいる")` →
    /// `end=4: (("ん" :NAI-N NIL))`
    /// `end=6: (("い" :TEIRU #<KANA-TEXT 2258170 い ord=0 common=NULL>))`
    /// `end=7: (("る" :TEIRU #<KANA-TEXT 1577980 いる ord=0 common=0>)
    ///          ("いる" :TEIRU+ #<KANA-TEXT 1577980 いる ord=0 common=0>))`.
    /// Exercises the `kf = None` case (`:NAI-N`) and the `:TEIRU+`
    /// class, plus two matches sharing end=7.
    #[tokio::test]
    async fn yondeiru_none_kf_and_teiru_plus() {
        let ctx = ctx_from_env().await;
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
