//! Port of `ichiran/dict:suffix-desho` (`dict-grammar.lisp:541`).
//!
//! Handles ～でしょ after a negative: when the root ends in ない, looks
//! up a word whose conjugation is negated.

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_prop::find_word_with_conj_prop;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_desho(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:542 (ends-with "ない" root)
    let primary_words: Vec<PrimaryWord> = if root.ends_with("ない") {
        // dict-grammar.lisp:543-544 (find-word-with-conj-prop root
        //   (lambda (cdata) (conj-neg (conj-data-prop cdata))))
        find_word_with_conj_prop(
            ctx,
            root,
            |cd| cd.prop.as_ref().is_some_and(|p| p.neg != Some(false)),
            false,
        )
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect()
    } else {
        Vec::new()
    };

    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Constant(300),
        connector: " ",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suf, kf, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::simple_text_class::SimpleText;

    /// `:desho` cache `kf` for "でしょう", REPL pinned: id=1122,
    /// seq=1008420, text="でしょう", ord=0, common=0,
    /// common_tags="[spec1]", conjugate_p=T, nokanji=NIL,
    /// best_kanji=:NULL. The `:desho` key also has a "でしょ" cache row
    /// (id=1123, ord=1) loaded by `(load-kf :desho (get-kana-form 1008420
    /// "でしょ"))` at `dict-grammar.lisp:271` — exercised by `desho4`.
    fn kf_deshou() -> KanaText {
        KanaText {
            id: 1122,
            seq: 1008420,
            text: "でしょう".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn kf_desho_short() -> KanaText {
        KanaText {
            id: 1123,
            seq: 1008420,
            text: "でしょ".into(),
            ord: 1,
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

    /// REPL: `(suffix-desho "食べない" "でしょう" kf)` → 1 COMPOUND
    /// text="食べないでしょう" kana="たべない でしょう"
    /// score-mod=(constantly 300) connector=" " primary=KANJI-TEXT
    /// (食べない seq 10092227).
    #[tokio::test]
    async fn desho1_nai_tail_kanji() {
        let ctx = ctx().await;
        let kf = kf_deshou();
        let result = suffix_desho(&ctx, "食べない", "でしょう", &kf)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べないでしょう");
        assert_eq!(c.kana, "たべない でしょう");
        assert!(matches!(c.score_mod, ScoreMod::Constant(300)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10092227),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-desho "ない" "でしょう" kf)` → 3 COMPOUNDs
    /// text="ないでしょう" each. Same 3 ない seqs as the desu test.
    #[tokio::test]
    async fn desho2_bare_nai() {
        let ctx = ctx().await;
        let kf = kf_deshou();
        let result = suffix_desho(&ctx, "ない", "でしょう", &kf).await.unwrap();
        assert_eq!(result.len(), 3);
        for c in &result {
            assert_eq!(c.text, "ないでしょう");
            assert_eq!(c.kana, "ない でしょう");
            assert!(matches!(c.score_mod, ScoreMod::Constant(300)));
            assert!(c.score_base.is_none());
            assert_eq!(c.words.len(), 2);
        }
    }

    /// REPL: `(suffix-desho "行かない" "でしょう" kf)` → 1 COMPOUND
    /// text="行かないでしょう" kana="いかない でしょう" primary=KANJI-TEXT
    /// (行かない id=922665 seq=10349392).
    #[tokio::test]
    async fn desho3_ikanai() {
        let ctx = ctx().await;
        let kf = kf_deshou();
        let result = suffix_desho(&ctx, "行かない", "でしょう", &kf)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "行かないでしょう");
        assert_eq!(c.kana, "いかない でしょう");
        assert!(matches!(c.score_mod, ScoreMod::Constant(300)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 922665);
                assert_eq!(k.seq, 10349392);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-desho "ない" "でしょ" kf-short)` → 3 COMPOUNDs
    /// text="ないでしょ" each, kana="ない でしょ". Exercises the short
    /// "でしょ" `kf` (cache id=1123).
    #[tokio::test]
    async fn desho4_short_desho_kf() {
        let ctx = ctx().await;
        let kf = kf_desho_short();
        let result = suffix_desho(&ctx, "ない", "でしょ", &kf).await.unwrap();
        assert_eq!(result.len(), 3);
        for c in &result {
            assert_eq!(c.text, "ないでしょ");
            assert_eq!(c.kana, "ない でしょ");
            assert!(matches!(c.score_mod, ScoreMod::Constant(300)));
            assert!(c.score_base.is_none());
            assert_eq!(c.words.len(), 2);
        }
    }

    /// REPL: each of `"食べ"`, `"ありません"`, `"行かなかった"` → NIL.
    /// Unlike suffix-desu, suffix-desho only takes "ない" tails, so
    /// "なかった" tails fall through.
    #[tokio::test]
    async fn desho5_no_nai_tail_returns_empty() {
        let ctx = ctx().await;
        let kf = kf_deshou();
        for r in ["食べ", "ありません", "行かなかった"] {
            let result = suffix_desho(&ctx, r, "でしょう", &kf).await.unwrap();
            assert!(result.is_empty(), "expected NIL for root={:?}", r);
        }
    }
}
