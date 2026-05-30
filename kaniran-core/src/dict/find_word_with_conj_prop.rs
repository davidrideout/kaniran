//! Port of `ichiran/dict:find-word-with-conj-prop` (`dict-grammar.lisp:44`).
//!
//! ```lisp
//! (defun find-word-with-conj-prop (wordstr filter-fn &key allow-root)
//!   (loop for word in (find-word-full wordstr)
//!        for conj-data = (word-conj-data word)
//!        for conj-data-filtered = (remove-if-not filter-fn conj-data)
//!        for conj-ids = (mapcar (lambda (cdata) (conj-id (conj-data-prop cdata))) conj-data-filtered)
//!        when (or conj-data-filtered (and (null conj-data) allow-root))
//!        do (setf (word-conjugations word) conj-ids)
//!        and collect word))
//! ```
//!
//! For each word from [`find_word_full`], read its conjugation data
//! through [`word_conj_data`], filter with the caller's predicate,
//! map the survivors to their `conj_id` values, and either (a) the
//! filtered list is non-empty, or (b) the original conj-data was
//! empty and `allow_root` is set — in both cases the word's
//! `word-conjugations` slot is mutated to the conj-ids list and the
//! word is collected.
//!
//! ## Divergences from Lisp
//!
//! - **Ctx-injected** per CONVENTIONS §4.8.
//! - **`filter_fn` typed as a [`FnMut`] borrowed closure** instead of
//!   a Lisp function value. Mirrors the upstream callsite usage where
//!   the lambda is constructed inline (`find-word-with-conj-type`,
//!   `suffix-sou-base`) — no upstream callsite reuses the same
//!   `filter-fn` across multiple invocations.
//! - **`allow_root` is `bool`** per CONVENTIONS §4.4 (binary `&key`
//!   keyword with a clearly-named boolean polarity).
//! - **`conj-id` of a `nil` prop is impossible** under the
//!   [`get_conj_data`] invariant (every emitted [`ConjData`] carries
//!   `Some(prop)`); the filter closure is the only path that could
//!   observe a `None` prop, and a closure that returns `true` for a
//!   `None` prop is a caller bug.
//!
//! ## word-conjugations slot mutation
//!
//! Upstream calls `(setf (word-conjugations word) conj-ids)` even when
//! `conj-ids` is the empty list (`mapcar` over `nil` yields `nil`).
//! In Rust:
//!
//! - non-empty conj-ids → `Some(WordConjugations::Ids(ids))`
//! - empty conj-ids (`allow_root` path with no upstream conj-data) →
//!   `None`
//!
//! The setter dispatch ([`set_word_conjugations`]) descends into
//! compound-text / proxy-text. The find-word-full output for this
//! callsite never includes counter-text (no `:counter` arg passed),
//! so the counter-text panic arm is unreachable here.
//!
//! [`find_word_full`]: super::find_word_full::find_word_full
//! [`word_conj_data`]: super::word_conj_data::word_conj_data
//! [`get_conj_data`]: super::get_conj_data::get_conj_data
//! [`ConjData`]: super::conj_data_struct::ConjData
//! [`set_word_conjugations`]: super::set_word_conjugations::set_word_conjugations

use crate::conn::kani_context::KaniranContext;
use crate::dict::conj_data_struct::ConjData;
use crate::dict::find_word_full::find_word_full;
use crate::dict::kani::KaniWordDispatchEnum;
use crate::dict::set_word_conjugations::set_word_conjugations;
use crate::dict::simple_text_class::WordConjugations;
use crate::dict::word_conj_data::word_conj_data;

pub async fn find_word_with_conj_prop<F>(
    ctx: &KaniranContext,
    wordstr: &str,
    mut filter_fn: F,
    allow_root: bool,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error>
where
    F: FnMut(&ConjData) -> bool,
{
    let words = find_word_full(ctx, wordstr, false, None).await?;
    let mut out: Vec<KaniWordDispatchEnum> = Vec::new();
    for mut word in words {
        // dict-grammar.lisp:45 (word-conj-data word)
        let conj_data = word_conj_data(ctx, &word).await?;
        // dict-grammar.lisp:46 (remove-if-not filter-fn conj-data)
        let filtered: Vec<&ConjData> = conj_data.iter().filter(|cd| filter_fn(cd)).collect();
        // dict-grammar.lisp:47 (mapcar (lambda (cdata) (conj-id (conj-data-prop cdata))) ...)
        let conj_ids: Vec<i32> = filtered
            .iter()
            .filter_map(|cd| cd.prop.as_ref().map(|p| p.conj_id))
            .collect();
        // dict-grammar.lisp:48 (when (or conj-data-filtered (and (null conj-data) allow-root))
        let allow_root_path = conj_data.is_empty() && allow_root;
        if !filtered.is_empty() || allow_root_path {
            // dict-grammar.lisp:49 (setf (word-conjugations word) conj-ids)
            // conj_ids may be empty (mapcar over nil = nil) — preserve
            // the upstream "nil-set" by passing None.
            let new_value = if conj_ids.is_empty() {
                None
            } else {
                Some(WordConjugations::Ids(conj_ids))
            };
            set_word_conjugations(&mut word, new_value);
            out.push(word);
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

    /// REPL: `(find-word-with-conj-prop "食べて" (lambda (cd) t))` →
    /// 1 word, allow_root=nil. Filter accepts every cdata.
    #[tokio::test]
    async fn t1_all_pass_no_allow_root() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(&ctx, "食べて", |_| true, false)
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 10092233);
        assert_eq!(
            k.state.conjugations,
            Some(WordConjugations::Ids(vec![92707]))
        );
    }

    /// REPL: `(find-word-with-conj-prop "食べて" (lambda (cd) t)
    /// :allow-root t)` → 1 word. allow_root doesn't change the
    /// outcome when conj-data is non-empty.
    #[tokio::test]
    async fn t2_all_pass_with_allow_root() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(&ctx, "食べて", |_| true, true)
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
    }

    /// REPL: `(find-word-with-conj-prop "食べる" (lambda (cd) t)
    /// :allow-root t)` → 1 word, wc=NIL. 食べる is a root: empty
    /// conj-data + allow_root → collect with conj_ids=nil.
    #[tokio::test]
    async fn t3_root_passthrough_with_allow_root() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(&ctx, "食べる", |_| true, true)
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 1358280);
        // Empty mapcar over nil → setter called with nil → None.
        assert_eq!(k.state.conjugations, None);
    }

    /// REPL: `(find-word-with-conj-prop "食べる" (lambda (cd) t))` →
    /// NIL. Without allow_root, root word is filtered out.
    #[tokio::test]
    async fn t4_root_dropped_without_allow_root() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(&ctx, "食べる", |_| true, false)
            .await
            .unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-conj-prop "食べて" (lambda (cd) nil))` →
    /// NIL. Filter rejects everything; without allow_root, no
    /// collection.
    #[tokio::test]
    async fn t5_reject_all() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(&ctx, "食べて", |_| false, false)
            .await
            .unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-conj-prop "食べて" (lambda (cd) nil)
    /// :allow-root t)` → NIL. Filter rejects all + word has conj-data
    /// → not the empty-conj-data-allow-root branch.
    #[tokio::test]
    async fn t6_reject_all_with_allow_root_keeps_filtering() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(&ctx, "食べて", |_| false, true)
            .await
            .unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-conj-prop "食べる" (lambda (cd) nil)
    /// :allow-root t)` → 1 word. Reject-all but conj-data empty +
    /// allow_root fires.
    #[tokio::test]
    async fn t7_reject_all_empty_conj_data_allow_root() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(&ctx, "食べる", |_| false, true)
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 1358280);
        assert_eq!(k.state.conjugations, None);
    }

    /// REPL: `(find-word-with-conj-prop "食べなくて"
    ///   (lambda (cd) (conj-neg (conj-data-prop cd))))` → 1 word
    /// (neg = T, BOOLEAN).
    /// Filter mirrors Lisp truthiness for `(conj-neg ...)`: in CL only
    /// `nil` is falsy, so both `t` and `:NULL` count. Translated to
    /// `Option<bool>` that means `p.neg != Some(false)` (None / :NULL
    /// → truthy per memory `feedback_null_nil_truthy`).
    #[tokio::test]
    async fn t8_neg_filter_matches() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(
            &ctx,
            "食べなくて",
            |cd| cd.prop.as_ref().is_some_and(|p| p.neg != Some(false)),
            false,
        )
        .await
        .unwrap();
        assert_eq!(r.len(), 1);
    }
}
