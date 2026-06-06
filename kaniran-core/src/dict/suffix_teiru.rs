//! Port of `ichiran/dict:suffix-teiru` (`dict-grammar.lisp:395`).
//!
//! Handles the ～ている progressive auxiliary: looks up the root via
//! `teiru-check`.

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::teiru_check::teiru_check;

pub async fn suffix_teiru(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:396 — (teiru-check root)
    let primary_words: Vec<PrimaryWord> = teiru_check(ctx, root)
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:395 — (:connector "" :score 3), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(3),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::simple_text_class::SimpleText;

    /// `:teiru` suffix-cache `kf` for "いる", REPL pinned via the
    /// いる(る) loop at `dict-grammar.lisp:210-215` against
    /// `(get-kana-forms 1577980)`: id=65814, seq=1577980, text="いる",
    /// common=0, common_tags="[ichi1]", conjugate_p=T, nokanji=nil,
    /// best_kanji="居る".
    fn kf_teiru_iru() -> KanaText {
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

    /// REPL TEIRU1: `(suffix-teiru "食べて" "る" kf-teiru-iru)` → 1
    /// COMPOUND text="食べてる" kana="たべてる" score-mod=3
    /// score-base=NIL primary=KANJI-TEXT (食べて seq 10092233).
    #[tokio::test]
    async fn teiru1_tabete_ru() {
        let ctx = ctx().await;
        let kf = kf_teiru_iru();
        let result = suffix_teiru(&ctx, "食べて", "る", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べてる");
        assert_eq!(c.kana, "たべてる");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
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

    /// REPL TEIRU2: `(suffix-teiru "いて" "る" kf-teiru-iru)` → NIL.
    /// teiru-check's `(not (equal root "いて"))` guard excludes bare
    /// いて.
    #[tokio::test]
    async fn teiru2_ite_excluded() {
        let ctx = ctx().await;
        let kf = kf_teiru_iru();
        let result = suffix_teiru(&ctx, "いて", "る", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL TEIRU3: `(suffix-teiru "で" "る" kf-teiru-iru)` → NIL.
    /// teiru-check delegates to te-check whose `(not (equal root "で"))`
    /// guard fires.
    #[tokio::test]
    async fn teiru3_de_excluded_via_te_check() {
        let ctx = ctx().await;
        let kf = kf_teiru_iru();
        let result = suffix_teiru(&ctx, "で", "る", &kf).await.unwrap();
        assert!(result.is_empty());
    }
}
