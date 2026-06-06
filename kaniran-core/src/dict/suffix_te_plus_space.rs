//! Port of `ichiran/dict:suffix-te+space` (`dict-grammar.lisp:401`).
//!
//! Handles a te-form followed by a space-joined auxiliary (くれる/もらう/
//! いただく): looks up the root via `te-check`.

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::te_check::te_check;

pub async fn suffix_te_plus_space(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:402 — (te-check root)
    let primary_words: Vec<PrimaryWord> = te_check(ctx, root)
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:401 — (:connector " " :score 3), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(3),
        connector: " ",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::simple_text_class::SimpleText;

    /// `:te+space` suffix-cache `kf` for "くれる", REPL pinned via
    /// `(get-kana-form 1269130 …)`: id=33764, seq=1269130, text="くれる",
    /// common=0, common_tags="[ichi1]", conjugate_p=T, nokanji=nil,
    /// best_kanji="呉れる".
    fn kf_te_plus_space_kureru() -> KanaText {
        KanaText {
            id: 33764,
            seq: 1269130,
            text: "くれる".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[ichi1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: Some("呉れる".into()),
            state: SimpleText::default(),
        }
    }

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL TESPACE1: `(suffix-te+space "食べて" "くれる" kf-kureru)` →
    /// 1 COMPOUND text="食べてくれる" kana="たべて くれる" score-mod=3
    /// score-base=NIL primary=KANJI-TEXT (食べて seq 10092233). Note
    /// the space between primary kana and suffix.
    #[tokio::test]
    async fn tespace1_tabete_kureru() {
        let ctx = ctx().await;
        let kf = kf_te_plus_space_kureru();
        let result = suffix_te_plus_space(&ctx, "食べて", "くれる", &kf)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べてくれる");
        assert_eq!(c.kana, "たべて くれる");
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

    /// REPL TESPACE2: `(suffix-te+space "で" "くれる" kf-kureru)` →
    /// NIL. te-check's `(not (equal root "で"))` guard fires.
    #[tokio::test]
    async fn tespace2_bare_de_excluded() {
        let ctx = ctx().await;
        let kf = kf_te_plus_space_kureru();
        let result = suffix_te_plus_space(&ctx, "で", "くれる", &kf)
            .await
            .unwrap();
        assert!(result.is_empty());
    }
}
