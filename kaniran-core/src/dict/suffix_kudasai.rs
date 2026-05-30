//! Port of `ichiran/dict:suffix-kudasai` (`dict-grammar.lisp:404`).
//!
//! ```lisp
//! (def-simple-suffix suffix-kudasai :kudasai (:connector " " :score (constantly 360)) (root)
//!   (te-check root))
//! ```
//!
//! `:score (constantly 360)` → [`ScoreMod::Constant(360)`].
//! `suf` typed `&KanaText`: `(load-kf :kudasai (get-kana-form 1184270
//! "ください" :conj :root))` materializes a kana-text.

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::te_check::te_check;

pub async fn suffix_kudasai(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:405 — (te-check root)
    let primary_words: Vec<PrimaryWord> = te_check(ctx, root)
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:404 — (:connector " " :score (constantly 360)), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Constant(360),
        connector: " ",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::simple_text_class::SimpleText;

    /// `:kudasai` suffix-cache `kf` for "ください", REPL pinned via
    /// `(get-kana-form 1184270 "ください" :conj :root)`: id=25048,
    /// seq=1184270, text="ください", common=13,
    /// common_tags="[news1][nf13]", conjugate_p=T, nokanji=nil,
    /// best_kanji="下さい".
    fn kf_kudasai() -> KanaText {
        KanaText {
            id: 25048,
            seq: 1184270,
            text: "ください".into(),
            ord: 0,
            common: Some(13),
            common_tags: "[news1][nf13]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: Some("下さい".into()),
            state: SimpleText::default(),
        }
    }

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL KUDASAI1: `(suffix-kudasai "食べて" "ください" kf-kudasai)`
    /// → 1 COMPOUND text="食べてください" kana="たべて ください"
    /// score-mod=#<FUNCTION (constantly 360)> score-base=NIL
    /// primary=KANJI-TEXT (食べて seq 10092233).
    #[tokio::test]
    async fn kudasai1_tabete_kudasai() {
        let ctx = ctx().await;
        let kf = kf_kudasai();
        let result = suffix_kudasai(&ctx, "食べて", "ください", &kf)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べてください");
        assert_eq!(c.kana, "たべて ください");
        assert!(matches!(c.score_mod, ScoreMod::Constant(360)));
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

    /// REPL KUDASAI2: `(suffix-kudasai "で" "ください" kf-kudasai)` →
    /// NIL. te-check excludes bare で.
    #[tokio::test]
    async fn kudasai2_bare_de_excluded() {
        let ctx = ctx().await;
        let kf = kf_kudasai();
        let result = suffix_kudasai(&ctx, "で", "ください", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL KUDASAI3: `(suffix-kudasai "食べる" "ください" kf-kudasai)`
    /// → NIL. te-check's last-char-in-"てで" guard fails.
    #[tokio::test]
    async fn kudasai3_last_char_not_te_or_de() {
        let ctx = ctx().await;
        let kf = kf_kudasai();
        let result = suffix_kudasai(&ctx, "食べる", "ください", &kf)
            .await
            .unwrap();
        assert!(result.is_empty());
    }
}
