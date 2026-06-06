//! Port of `ichiran/dict:get-suffixes` (`dict-grammar.lisp:682`).
//!
//! Enumerates every grammatical-suffix match anchored at any non-zero
//! offset of `word`, ordered so that matches at higher `start`
//! (shorter suffixes) come first.

use crate::conn::kani_context::KaniranContext;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::parse_suffix_val::parse_suffix_val;
use crate::dict::subseq_slice::subseq_slice;

pub fn get_suffixes<'a, 'b>(
    ctx: &'a KaniranContext,
    word: &'b str,
) -> Vec<(&'b str, &'a str, Option<&'a KanaText>)> {
    // dict-grammar.lisp:683 (init-suffixes) — eager-population invariant
    // of `KaniranContext::from_url`. The contract is load-bearing: an
    // unpopulated cache makes get_suffixes silently degenerate to the
    // empty answer for every word, which would corrupt every segmenter
    // path that depends on suffix recognition. Guard in release too.
    assert!(
        !ctx.suffix_cache.is_empty(),
        "get-suffixes: ctx.suffix_cache is empty (init-suffixes contract violated)",
    );
    let len = word.chars().count();
    let mut out: Vec<(&'b str, &'a str, Option<&'a KanaText>)> = Vec::new();
    // dict-grammar.lisp:684 (loop for start from (1- (length word)) downto 1)
    for start in (1..len).rev() {
        // dict-grammar.lisp:685 (subseq-slice nil word start)
        let substr = subseq_slice(None, word, start, None);
        // dict-grammar.lisp:686 (gethash substr *suffix-cache*)
        let val = ctx.suffix_cache.get(substr).map(|v| v.as_slice());
        // dict-grammar.lisp:687 (nconc (parse-suffix-val substr val))
        out.extend(parse_suffix_val(substr, val));
    }
    out
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

    /// REPL: `(get-suffixes "")` → `NIL`. Length-0 word: loop range
    /// `from -1 downto 1` is empty.
    #[tokio::test]
    async fn empty_word_yields_empty() {
        let ctx = ctx_from_env().await;
        let out = get_suffixes(&ctx, "");
        assert!(out.is_empty());
    }

    /// REPL: `(get-suffixes "あ")` → `NIL`. Length-1 word: loop range
    /// `from 0 downto 1` is empty.
    #[tokio::test]
    async fn single_char_word_yields_empty() {
        let ctx = ctx_from_env().await;
        let out = get_suffixes(&ctx, "あ");
        assert!(out.is_empty());
    }

    /// REPL: `(get-suffixes "る")` → `NIL`. Even when the trailing char
    /// has a cache entry, length-1 input never enters the loop body.
    #[tokio::test]
    async fn cached_substr_at_length_one_yields_empty() {
        let ctx = ctx_from_env().await;
        let out = get_suffixes(&ctx, "る");
        assert!(out.is_empty());
    }

    /// REPL: `(get-suffixes "abcde")` → `NIL`. ASCII word with no
    /// cached substrings — exercises the loop walking 4 starts with
    /// only `gethash` misses.
    #[tokio::test]
    async fn no_match_yields_empty() {
        let ctx = ctx_from_env().await;
        let out = get_suffixes(&ctx, "abcde");
        assert!(out.is_empty());
    }

    /// REPL: `(get-suffixes "食べてる")` →
    /// `(("る" :TEIRU #<KANA-TEXT 1577980 いる ord=0 common=0>))`.
    /// Single suffix match at the deepest start (start=3 → "る").
    #[tokio::test]
    async fn single_match_at_deepest_start() {
        let ctx = ctx_from_env().await;
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
    #[tokio::test]
    async fn multiple_matches_in_decreasing_start_order() {
        let ctx = ctx_from_env().await;
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
    #[tokio::test]
    async fn three_matches_shortest_suffix_first() {
        let ctx = ctx_from_env().await;
        let out = get_suffixes(&ctx, "食べてました");
        assert_eq!(out.len(), 3);
        let (s0, k0, kf0) = out[0];
        let (s1, k1, kf1) = out[1];
        let (s2, k2, kf2) = out[2];
        assert_eq!(s0, "た");
        assert_eq!(k0, "teiru");
        let kf0 = kf0.expect("kf0 row present");
        assert_eq!(kf0.seq, 10551837);
        assert_eq!(kf0.text, "いた");
        assert_eq!(kf0.ord, 0);
        assert_eq!(kf0.common, None);
        assert_eq!(s1, "した");
        assert_eq!(k1, "suru");
        let kf1 = kf1.expect("kf1 row present");
        assert_eq!(kf1.seq, 10152246);
        assert_eq!(kf1.text, "した");
        assert_eq!(kf1.ord, 0);
        assert_eq!(kf1.common, None);
        assert_eq!(s2, "ました");
        assert_eq!(k2, "teiru");
        let kf2 = kf2.expect("kf2 row present");
        assert_eq!(kf2.seq, 10551838);
        assert_eq!(kf2.text, "いました");
        assert_eq!(kf2.ord, 0);
        assert_eq!(kf2.common, None);
    }
}
