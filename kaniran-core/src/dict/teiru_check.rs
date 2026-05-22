//! Port of `ichiran/dict:teiru-check` (`dict-grammar.lisp:392`).
//!
//! ```lisp
//! (defun teiru-check (root)
//!   (and (not (equal root "いて")) (te-check root)))
//! ```
//!
//! Stricter variant of [`te_check`]: the literal "いて" is excluded
//! (the `:teiru` suffix would otherwise loop in on its own canonical
//! form `いる` → `いて`), then delegates to `te-check`. Used by
//! `suffix-teiru` and `suffix-teiru+` (`dict-grammar.lisp:395, 398`)
//! as both the predicate gate and the primary-words pile.
//!
//! Diverges from the upstream lambda list `(root)` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`]. The
//! upstream `(and …)` chain flows the right-hand value out when the
//! guard passes; the Rust port mirrors that by returning the
//! `te_check` result unchanged in the non-excluded path.

use crate::conn::kani_context::KaniranContext;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::te_check::te_check;

pub async fn teiru_check(
    ctx: &KaniranContext,
    root: &str,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    // dict-grammar.lisp:393 (not (equal root "いて"))
    if root == "いて" {
        return Ok(Vec::new());
    }
    te_check(ctx, root).await
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// REPL: `(teiru-check "いて")` → NIL. The "いて" guard excludes
    /// the literal canonical form of the iru suffix.
    #[tokio::test]
    async fn t1_ite_excluded() {
        let ctx = ctx().await;
        let r = teiru_check(&ctx, "いて").await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(teiru-check "で")` → NIL via te-check's bare-で guard.
    #[tokio::test]
    async fn t2_te_check_failure_propagates() {
        let ctx = ctx().await;
        let r = teiru_check(&ctx, "で").await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(teiru-check "食べる")` → NIL via te-check's "last char
    /// in てで" guard.
    #[tokio::test]
    async fn t3_no_te_or_de_ending() {
        let ctx = ctx().await;
        let r = teiru_check(&ctx, "食べる").await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(teiru-check "食べて")` → 1 word (delegates to
    /// te-check). Same fixture as `te_check::tests::t5`.
    #[tokio::test]
    async fn t4_tabete_delegates_to_te_check() {
        let ctx = ctx().await;
        let r = teiru_check(&ctx, "食べて").await.unwrap();
        assert_eq!(r.len(), 1);
        let crate::dict::kani_word::KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 10092233);
    }

    /// REPL: `(teiru-check "見て")` → 1 word.
    #[tokio::test]
    async fn t5_mite() {
        let ctx = ctx().await;
        let r = teiru_check(&ctx, "見て").await.unwrap();
        assert_eq!(r.len(), 1);
    }

    /// REPL: `(teiru-check "")` signals `SIMPLE-TYPE-ERROR` via the
    /// `te-check` delegation (`(char "" -1)` raises). Mirror via panic.
    #[tokio::test]
    #[should_panic(expected = "te-check: (char root (1- (length root))) on empty root signals upstream")]
    async fn t6_empty_root_panics_via_te_check() {
        let ctx = ctx().await;
        let _ = teiru_check(&ctx, "").await;
    }
}
