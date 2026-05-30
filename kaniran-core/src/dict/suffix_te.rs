//! Port of `ichiran/dict:suffix-te` (`dict-grammar.lisp:389`).
//!
//! ```lisp
//! (def-simple-suffix suffix-te :te (:connector "" :score 0) (root)
//!   (te-check root))
//! ```
//!
//! `suf` typed `&KanaText`: every `:te` cache populator (the
//! `(load-conjs :te …)` rows, the `いく/く` loop, `(load-kf :te …)`)
//! materializes a kana-text.

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::te_check::te_check;

pub async fn suffix_te(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:390 — (te-check root)
    let primary_words: Vec<PrimaryWord> = te_check(ctx, root)
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:389 — (:connector "" :score 0), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(0),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::simple_text_class::SimpleText;

    /// `:te` suffix-cache `kf` for "も", REPL pinned: `(get-kana-form
    /// 2028940 "も")` → id=110365, seq=2028940, text="も", common=0,
    /// common_tags="[spec1]", conjugate_p=T, nokanji=nil,
    /// best_kanji=:NULL.
    fn kf_te_mo() -> KanaText {
        KanaText {
            id: 110365,
            seq: 2028940,
            text: "も".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL TE1: `(suffix-te "食べて" "おる" kf-te-mo)` → 1 COMPOUND
    /// text="食べておる" kana="たべておる" score-mod=0 score-base=NIL
    /// primary=KANJI-TEXT (食べて seq 10092233). Hits the te-check
    /// te-ending arm (root last char て).
    #[tokio::test]
    async fn te1_tabete_oru() {
        let ctx = ctx().await;
        let kf = kf_te_mo();
        let result = suffix_te(&ctx, "食べて", "おる", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べておる");
        assert_eq!(c.kana, "たべておる");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べて");
                assert_eq!(k.seq, 10092233);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL TE2: `(suffix-te "で" "おって" kf-te-mo)` → NIL. te-check's
    /// `(not (equal root "で"))` guard excludes bare で.
    #[tokio::test]
    async fn te2_bare_de_excluded() {
        let ctx = ctx().await;
        let kf = kf_te_mo();
        let result = suffix_te(&ctx, "で", "おって", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL TE3: `(suffix-te "食べる" "おって" kf-te-mo)` → NIL. Last
    /// char る not in "てで", te-check's second guard fails.
    #[tokio::test]
    async fn te3_last_char_not_te_or_de() {
        let ctx = ctx().await;
        let kf = kf_te_mo();
        let result = suffix_te(&ctx, "食べる", "おって", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL TE4: `(suffix-te "食べ" "おって" kf-te-mo)` → NIL. Root
    /// 食べ ends in べ; te-check's second guard fails.
    #[tokio::test]
    async fn te4_stem_last_char_not_te_or_de() {
        let ctx = ctx().await;
        let kf = kf_te_mo();
        let result = suffix_te(&ctx, "食べ", "おって", &kf).await.unwrap();
        assert!(result.is_empty());
    }
}
