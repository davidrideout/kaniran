//! Port of `ichiran/dict:find-word-with-conj-type` (`dict-grammar.lisp:53`).
//!
//! Delegates to [`find_word_with_conj_prop`] with a filter that admits
//! any conj-data whose prop's `conj-type` is in the caller-supplied
//! integer set.
//!
//! [`find_word_with_conj_prop`]: super::find_word_with_conj_prop::find_word_with_conj_prop

use crate::conn::kani_context::KaniranContext;
use crate::dict::find_word_with_conj_prop::find_word_with_conj_prop;
use crate::dict::kani_word::KaniWordDispatchEnum;

pub async fn find_word_with_conj_type(
    ctx: &KaniranContext,
    word: &str,
    conj_types: &[i32],
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    find_word_with_conj_prop(
        ctx,
        word,
        // dict-grammar.lisp:54-56 — (lambda (cdata) (member (conj-type (conj-data-prop cdata)) conj-types))
        // Lisp `(member x nil)` is nil — closing over an empty slice
        // returns false for every cdata, mirroring (member … '()).
        |cd| {
            cd.prop
                .as_ref()
                .is_some_and(|p| conj_types.contains(&p.conj_type))
        },
        false,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// REPL: `(find-word-with-conj-type "食べて" 3)` → 1 word
    /// text=食べて seq=10092233 wc=(92707).
    #[tokio::test]
    async fn t1_conj_type_3_matches() {
        let ctx = ctx().await;
        let r = find_word_with_conj_type(&ctx, "食べて", &[3]).await.unwrap();
        assert_eq!(r.len(), 1);
        let crate::dict::kani_word::KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 10092233);
        assert_eq!(
            k.state.conjugations,
            Some(crate::dict::simple_text_class::WordConjugations::Ids(vec![92707]))
        );
    }

    /// REPL: `(find-word-with-conj-type "食べ" 13)` → 1 word
    /// text=食べ seq=10092273 wc=(92747). Type 13 is ren'youkei stem.
    #[tokio::test]
    async fn t2_conj_type_13() {
        let ctx = ctx().await;
        let r = find_word_with_conj_type(&ctx, "食べ", &[13]).await.unwrap();
        assert_eq!(r.len(), 1);
        let crate::dict::kani_word::KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 10092273);
    }

    /// REPL: `(find-word-with-conj-type "食べる" 3)` → NIL. 食べる is a
    /// root, not a -te form.
    #[tokio::test]
    async fn t3_no_match_for_root() {
        let ctx = ctx().await;
        let r = find_word_with_conj_type(&ctx, "食べる", &[3]).await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-conj-type "食べ" 3 13)` → 1 word (type
    /// 13 hits; type 3 doesn't). Exercises the multi-conj-type set
    /// `(member x '(3 13))` arm.
    #[tokio::test]
    async fn t4_multi_conj_types() {
        let ctx = ctx().await;
        let r = find_word_with_conj_type(&ctx, "食べ", &[3, 13])
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
    }

    /// REPL: `(find-word-with-conj-type "ジャバスクリプト" 3)` → NIL.
    /// Word with no conjugations.
    #[tokio::test]
    async fn t5_no_conj_data() {
        let ctx = ctx().await;
        let r = find_word_with_conj_type(&ctx, "ジャバスクリプト", &[3])
            .await
            .unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-conj-type "abc" 3)` → NIL. No dictionary
    /// entry at all.
    #[tokio::test]
    async fn t6_no_entry() {
        let ctx = ctx().await;
        let r = find_word_with_conj_type(&ctx, "abc", &[3]).await.unwrap();
        assert!(r.is_empty());
    }

    /// Empty `conj_types` mirrors `(find-word-with-conj-type "食べて")`
    /// — the closure `(member x nil)` is nil for every cdata; filter
    /// drops everything; allow_root=false; nothing collected.
    #[tokio::test]
    async fn t7_empty_conj_types() {
        let ctx = ctx().await;
        let r = find_word_with_conj_type(&ctx, "食べて", &[]).await.unwrap();
        assert!(r.is_empty());
    }
}
