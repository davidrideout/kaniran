//! Port of `ichiran/dict:find-word-with-suffix` (`dict-grammar.lisp:102`).
//!
//! ```lisp
//! (defun find-word-with-suffix (wordstr &rest suffix-classes)
//!   (loop for word in (find-word-full wordstr)
//!      for seq = (seq word)
//!      for suffix-class = (and (listp seq) (gethash (car (last seq)) *suffix-class*))
//!      when (and suffix-class (find suffix-class suffix-classes)) collect word))
//! ```
//!
//! Collects the [`find_word_full`] outputs whose `seq` is a list
//! (Lisp `(listp seq)` is true only for compound-text via
//! `(mapcar #'seq (words obj))`) and whose last element's seq maps to
//! one of `suffix_classes` in [`SUFFIX_CLASS`]. Used by the suffix
//! grammar (e.g. `suffix-sa`, `suffix-ren`) to recognize an existing
//! compound formed against a particular suffix class without
//! materializing the entire suffix expansion path again.
//!
//! ## Divergences from Lisp
//!
//! - **Ctx-injected** per CONVENTIONS §4.8.
//! - **`suffix_classes` as `&[&str]`** instead of `&rest` packed
//!   positional; suffix-class values are the lowercase keyword
//!   strings the cache uses (`"ra"`, `"suru"`, …) per
//!   [`super::_star_suffix_class_star_`].
//! - **`(listp seq)`** maps to `WordInfoSeq::Multi(_)`: per the
//!   [`super::seq::seq`] dispatcher, only compound-text yields a
//!   `Multi` value. `Single` (simple-text / counter-text-with-source)
//!   and `None` (sourceless counter-text) both skip the suffix-class
//!   lookup, matching upstream's `(listp seq)` guard.
//! - **Nested-compound `(car (last seq))`** would be a sublist in
//!   Lisp (and the hash lookup misses); in Rust the `Multi` payload's
//!   last element being itself `Some(Multi(_))` produces no lookup
//!   key and the word is skipped — same observable outcome.
//!
//! [`find_word_full`]: super::find_word_full::find_word_full
//! [`SUFFIX_CLASS`]: super::_star_suffix_class_star_::suffix_class

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_suffix_class_star_::suffix_class;
use crate::dict::find_word_full::find_word_full;
use crate::dict::kani::KaniWordDispatchEnum;
use crate::dict::seq::seq;
use crate::dict::word_info_class::WordInfoSeq;

pub async fn find_word_with_suffix(
    ctx: &KaniranContext,
    wordstr: &str,
    suffix_classes: &[&str],
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let words = find_word_full(ctx, wordstr, false, None).await?;
    let class_map = suffix_class(ctx);
    let mut out: Vec<KaniWordDispatchEnum> = Vec::new();
    for word in words {
        // dict-grammar.lisp:104 (seq word)
        let word_seq = seq(&word);
        // dict-grammar.lisp:105 (and (listp seq) (gethash (car (last seq)) *suffix-class*))
        let class: Option<&String> = match word_seq {
            Some(WordInfoSeq::Multi(elems)) => match elems.last() {
                Some(Some(WordInfoSeq::Single(i))) => class_map.get(i),
                // nested compound or nil last element — hash lookup misses
                _ => None,
            },
            _ => None,
        };
        // dict-grammar.lisp:106 (when (and suffix-class (find suffix-class suffix-classes)) collect word)
        if let Some(cls) = class {
            if suffix_classes.contains(&cls.as_str()) {
                out.push(word);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// REPL: `(find-word-with-suffix "我々ら" :ra)` → 1 compound
    /// text=我々ら kana=われわれら.
    #[tokio::test]
    async fn t1_warera_ra_match() {
        let ctx = ctx().await;
        let r = find_word_with_suffix(&ctx, "我々ら", &["ra"]).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected COMPOUND");
        };
        assert_eq!(c.text, "我々ら");
        assert_eq!(c.kana, "われわれら");
    }

    /// REPL: `(find-word-with-suffix "勉強する" :suru)` → 1 compound
    /// text=勉強する kana=べんきょう する.
    #[tokio::test]
    async fn t2_benkyousuru_suru_match() {
        let ctx = ctx().await;
        let r = find_word_with_suffix(&ctx, "勉強する", &["suru"])
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected COMPOUND");
        };
        assert_eq!(c.text, "勉強する");
        assert_eq!(c.kana, "べんきょう する");
    }

    /// REPL: `(find-word-with-suffix "勉強する" :ra)` → NIL (class
    /// mismatch).
    #[tokio::test]
    async fn t3_wrong_class_drops() {
        let ctx = ctx().await;
        let r = find_word_with_suffix(&ctx, "勉強する", &["ra"]).await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-suffix "区別" :suru)` → NIL. Simple-text
    /// `seq` is an integer (not listp) — class lookup skipped.
    #[tokio::test]
    async fn t4_simple_text_seq_not_listp() {
        let ctx = ctx().await;
        let r = find_word_with_suffix(&ctx, "区別", &["suru"]).await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-suffix "abc" :suru)` → NIL.
    #[tokio::test]
    async fn t5_no_entries() {
        let ctx = ctx().await;
        let r = find_word_with_suffix(&ctx, "abc", &["suru"]).await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-suffix "勉強する")` → NIL. Empty
    /// suffix-classes — `(find x nil)` is always nil → no
    /// collection.
    #[tokio::test]
    async fn t6_empty_classes() {
        let ctx = ctx().await;
        let r = find_word_with_suffix(&ctx, "勉強する", &[]).await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-suffix "勉強する" :ra :suru)` → 1
    /// compound (suru matches, ra doesn't). Multi-class set.
    #[tokio::test]
    async fn t7_multi_class_set() {
        let ctx = ctx().await;
        let r = find_word_with_suffix(&ctx, "勉強する", &["ra", "suru"])
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
    }
}
