//! Port of `ichiran/dict:te-check` (`dict-grammar.lisp:384`).
//!
//! ```lisp
//! (defun te-check (root)
//!   (and (not (equal root "で"))
//!        (find (char root (1- (length root))) "てで")
//!        (find-word-with-conj-type root 3)))
//! ```
//!
//! Three-clause `and`: bare "で" is excluded; otherwise the last
//! character must be て or で; otherwise the result is whatever
//! [`find_word_with_conj_type`] returns for `(root 3)` — conjugation
//! type 3 is the -te form. Returns the word list (empty when any
//! clause fails) so callsites in `suffix-te`, `suffix-teiru`,
//! `suffix-te+space`, `suffix-kudasai`, `suffix-te-ren`, `suffix-teii`
//! can use it both as a predicate and as the primary-words pile.
//!
//! Diverges from the upstream lambda list `(root)` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`]. The
//! upstream `(and …)` chain whose value flows through is preserved by
//! returning an empty `Vec` when any guard fails — `Vec::is_empty()`
//! at the callsite reads the same as the upstream `nil`-as-falsy
//! check, and the truthy branch hands back the
//! `find-word-with-conj-type` rows unchanged.

use crate::conn::kani_context::KaniranContext;
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::kani_word::KaniWordDispatchEnum;

pub async fn te_check(
    ctx: &KaniranContext,
    root: &str,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    // dict-grammar.lisp:385 (not (equal root "で"))
    if root == "で" {
        return Ok(Vec::new());
    }
    // dict-grammar.lisp:386 (find (char root (1- (length root))) "てで")
    // (char "" -1) signals SIMPLE-TYPE-ERROR upstream; mirror via panic
    // so a caller passing empty root sees an equivalent abort rather
    // than a silent empty result. Dead in practice — find-word-suffix
    // gates the callsite with (> offset 0) so root is never empty.
    let last = root
        .chars()
        .last()
        .expect("te-check: (char root (1- (length root))) on empty root signals upstream");
    if last != 'て' && last != 'で' {
        return Ok(Vec::new());
    }
    // dict-grammar.lisp:387 (find-word-with-conj-type root 3)
    find_word_with_conj_type(ctx, root, &[3]).await
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// REPL: `(te-check "で")` → NIL. Bare "で" is excluded by the
    /// first `(not (equal root "で"))` guard.
    #[tokio::test]
    async fn t1_bare_de_excluded() {
        let ctx = ctx().await;
        let r = te_check(&ctx, "で").await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(te-check "食べる")` → NIL. Last char る not in "てで",
    /// so the second guard fails before find-word-with-conj-type runs.
    #[tokio::test]
    async fn t2_last_char_not_te_or_de() {
        let ctx = ctx().await;
        let r = te_check(&ctx, "食べる").await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(te-check "")` signals `SIMPLE-TYPE-ERROR: Invalid index -1
    /// for (SIMPLE-ARRAY CHARACTER (0))` — `(char "" -1)` raises rather
    /// than returning. Mirror via panic.
    #[tokio::test]
    #[should_panic(expected = "te-check: (char root (1- (length root))) on empty root signals upstream")]
    async fn t3_empty_root_panics_like_upstream() {
        let ctx = ctx().await;
        let _ = te_check(&ctx, "").await;
    }

    /// REPL: `(te-check "空")` → NIL. Last char 空 not in "てで".
    #[tokio::test]
    async fn t4_kanji_last_char() {
        let ctx = ctx().await;
        let r = te_check(&ctx, "空").await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(te-check "食べて")` → 1 word: text=食べて seq=10092233
    /// type=KANJI-TEXT wc=(92707) — conj-id 92707 is the -te form
    /// (conj-type 3) of 食べる (seq 1358280).
    #[tokio::test]
    async fn t5_tabete_succeeds() {
        let ctx = ctx().await;
        let r = te_check(&ctx, "食べて").await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT, got {:?}", r[0]);
        };
        assert_eq!(k.text, "食べて");
        assert_eq!(k.seq, 10092233);
        assert_eq!(
            k.state.conjugations,
            Some(crate::dict::simple_text_class::WordConjugations::Ids(vec![92707]))
        );
    }

    /// REPL: `(te-check "遊んで")` → 2 words (last char で). Verifies
    /// the で path (not just て).
    #[tokio::test]
    async fn t6_asonde_de_last_char() {
        let ctx = ctx().await;
        let r = te_check(&ctx, "遊んで").await.unwrap();
        assert_eq!(r.len(), 2);
    }

    /// REPL: `(te-check "見て")` → 1 word.
    #[tokio::test]
    async fn t7_mite_te_last_char() {
        let ctx = ctx().await;
        let r = te_check(&ctx, "見て").await.unwrap();
        assert_eq!(r.len(), 1);
    }
}
