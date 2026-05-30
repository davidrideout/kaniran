//! Port of `ichiran/dict:abbr-nx` (`dict-grammar.lisp:591-600`).
//!
//! ```lisp
//! (def-abbr-suffix abbr-nx :nai-x 2 (root suf patch)
//!   (cond ((equal root "せ")
//!          (setf patch '("しない" . "せ"))
//!          (find-word-conj-of "しない" 1157170))
//!         (t
//!          (find-word-with-conj-prop
//!           (concatenate 'string root "ない")
//!           (lambda (cdata)
//!             (and (/= (conj-data-from cdata) 1157170)
//!                  (conj-neg (conj-data-prop cdata))))))))
//! ```
//!
//! Mapcar tail delegated to [`def_abbr_suffix_body`] (CONVENTIONS
//! §4.6 case (c)). `patch` (cons cell `("しない" . "せ")`) flows as
//! `Some(("しない", "せ"))` into the body's kana branch:
//! `destem(source-kana, 3) + "せ" + suf-var` (3 = char-count of
//! `(car patch)` = "しない").

use crate::conn::kani_context::KaniranContext;
use crate::dict::def_abbr_suffix_macro::def_abbr_suffix_body;
use crate::dict::find_word_conj_of::find_word_conj_of;
use crate::dict::find_word_seq::WordSeqRows;
use crate::dict::find_word_with_conj_prop::find_word_with_conj_prop;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani::KaniWordDispatchEnum;

pub async fn abbr_nx(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    // dict-grammar.lisp:555 — (let* ((*suffix-map-temp* nil) …))
    let ctx_rebound = ctx.with_suffix_map_temp(None);

    // dict-grammar.lisp:592-600 (cond ((equal root "せ") …) (t …))
    let (primary_words, patch): (Vec<KaniWordDispatchEnum>, Option<(&str, &str)>) =
        if root == "せ" {
            // dict-grammar.lisp:593-594 — (setf patch '("しない" . "せ"))
            //                              (find-word-conj-of "しない" 1157170)
            let rows = find_word_conj_of(&ctx_rebound, "しない", &[1157170]).await?;
            let words: Vec<KaniWordDispatchEnum> = match rows {
                WordSeqRows::Kana(v) => v.into_iter().map(KaniWordDispatchEnum::Kana).collect(),
                WordSeqRows::Kanji(v) => v.into_iter().map(KaniWordDispatchEnum::Kanji).collect(),
            };
            (words, Some(("しない", "せ")))
        } else {
            // dict-grammar.lisp:596-600 — (find-word-with-conj-prop (concatenate root "ない") λ)
            let wordstr = format!("{}{}", root, "ない");
            let words = find_word_with_conj_prop(
                &ctx_rebound,
                &wordstr,
                // dict-grammar.lisp:598-599 — (and (/= (conj-data-from cdata) 1157170)
                //                                   (conj-neg (conj-data-prop cdata)))
                |cdata| {
                    cdata.from != Some(1157170)
                        && cdata.prop.as_ref().is_some_and(|p| p.neg != Some(false))
                },
                false,
            )
            .await?;
            (words, None)
        };

    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, patch).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani::KaniSimpleTextDispatchEnum;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL NX1: `(abbr-nx "知ら" "ず" nil)` → 1 PROXY
    /// text="知らず" kana="しらず" hintedp=T source=KANJI-TEXT
    /// (seq 10105960, text "知らない"). Exercises the t-arm filter.
    #[tokio::test]
    async fn nx1_shira_zu_kanji() {
        let ctx = ctx().await;
        let result = abbr_nx(&ctx, "知ら", "ず", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "知らず");
        assert_eq!(p.kana, "しらず");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10105960);
    }

    /// REPL NX2: `(abbr-nx "食べ" "ず" nil)` → 1 PROXY
    /// text="食べず" kana="たべず" source=KANJI-TEXT seq=10092227.
    #[tokio::test]
    async fn nx2_tabe_zu_kanji() {
        let ctx = ctx().await;
        let result = abbr_nx(&ctx, "食べ", "ず", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "食べず");
        assert_eq!(p.kana, "たべず");
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10092227);
    }

    /// REPL NX3: `(abbr-nx "せ" "ず" nil)` → 1 PROXY text="せず"
    /// kana="せず" source=KANA-TEXT (seq=10152244, text "しない").
    /// Exercises the patch branch: `destem("しない", 3) = ""` +
    /// patch-cdr "せ" + suf-var "ず" → "せず".
    #[tokio::test]
    async fn nx3_se_zu_patch_branch() {
        let ctx = ctx().await;
        let result = abbr_nx(&ctx, "せ", "ず", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "せず");
        assert_eq!(p.kana, "せず");
        assert!(p.state.hintedp);
        assert_eq!(p.state.conjugations, None);
        let KaniSimpleTextDispatchEnum::Kana(k) = &*p.source else {
            panic!("expected Kana source");
        };
        assert_eq!(k.seq, 10152244);
        assert_eq!(k.text, "しない");
    }

    /// REPL NX4: `(abbr-nx "せ" "ざる" nil)` → 1 PROXY text="せざる"
    /// kana="せざる" (patch branch with different suf-var).
    #[tokio::test]
    async fn nx4_se_zaru_patch_branch() {
        let ctx = ctx().await;
        let result = abbr_nx(&ctx, "せ", "ざる", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "せざる");
        assert_eq!(p.kana, "せざる");
    }

    /// REPL NX5: `(abbr-nx "せ" "ぬ" nil)` → 1 PROXY text="せぬ"
    /// kana="せぬ" (patch branch with yet another suf-var).
    #[tokio::test]
    async fn nx5_se_nu_patch_branch() {
        let ctx = ctx().await;
        let result = abbr_nx(&ctx, "せ", "ぬ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "せぬ");
        assert_eq!(p.kana, "せぬ");
    }
}
