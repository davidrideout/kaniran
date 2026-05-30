//! Port of `ichiran/dict:suffix-teiru+` (`dict-grammar.lisp:398`).
//!
//! ```lisp
//! (def-simple-suffix suffix-teiru+ :teiru+ (:connector "" :score 6) (root)
//!   (teiru-check root))
//! ```
//!
//! `suf` typed `&KanaText`: the `いる(る)` loop at
//! `dict-grammar.lisp:210-215` populates kana-texts on the length-1+
//! branch.

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::teiru_check::teiru_check;

pub async fn suffix_teiru_plus_(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:399 — (teiru-check root)
    let primary_words: Vec<PrimaryWord> = teiru_check(ctx, root)
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:398 — (:connector "" :score 6), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(6),
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

    /// `:teiru+` suffix-cache `kf` for "いる", REPL pinned via the
    /// いる(る) loop at `dict-grammar.lisp:210-215`: id=65814,
    /// seq=1577980, text="いる", common=0, common_tags="[ichi1]",
    /// conjugate_p=T, nokanji=nil, best_kanji="居る".
    fn kf_teiru_plus_iru() -> KanaText {
        KanaText {
            id: 65814,
            seq: 1577980,
            text: "いる".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[ichi1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: Some("居る".into()),
            state: SimpleText::default(),
        }
    }

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL TEIRU+1: `(suffix-teiru+ "食べて" "いる" kf-teiru-plus-iru)`
    /// → 1 COMPOUND text="食べている" kana="たべている" score-mod=6
    /// score-base=NIL primary=KANJI-TEXT (食べて seq 10092233).
    #[tokio::test]
    async fn teiru_plus_1_tabete_iru() {
        let ctx = ctx().await;
        let kf = kf_teiru_plus_iru();
        let result = suffix_teiru_plus_(&ctx, "食べて", "いる", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べている");
        assert_eq!(c.kana, "たべている");
        assert!(matches!(c.score_mod, ScoreMod::Single(6)));
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

    /// REPL TEIRU+2: `(suffix-teiru+ "いて" "いる" kf-teiru-plus-iru)` →
    /// NIL. teiru-check's `(not (equal root "いて"))` guard excludes
    /// bare いて.
    #[tokio::test]
    async fn teiru_plus_2_ite_excluded() {
        let ctx = ctx().await;
        let kf = kf_teiru_plus_iru();
        let result = suffix_teiru_plus_(&ctx, "いて", "いる", &kf).await.unwrap();
        assert!(result.is_empty());
    }
}
