//! Port of `ichiran/dict:suffix-desu` (`dict-grammar.lisp:525`).
//!
//! ```lisp
//! (def-simple-suffix suffix-desu :desu (:connector " " :score (constantly 200)) (root)
//!   (and (or (alexandria:ends-with-subseq "ない" root)
//!            (alexandria:ends-with-subseq "なかった" root))
//!        (find-word-with-conj-prop root (lambda (cdata)
//!                                         (conj-neg (conj-data-prop cdata))))))
//! ```
//!
//! `:score (constantly 200)` → [`ScoreMod::Constant(200)`]. `:stem 0`
//! → outer macro rebind is a no-op. Mapcar tail delegated to
//! [`def_simple_suffix_body`].
//!
//! [`def_simple_suffix_body`]: super::def_simple_suffix_macro::def_simple_suffix_body
//! [`ScoreMod::Constant(200)`]: super::compound_text_class::ScoreMod::Constant

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_prop::find_word_with_conj_prop;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_desu(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:526-527 (or (ends-with "ない" root) (ends-with "なかった" root))
    let primary_words: Vec<PrimaryWord> =
        if root.ends_with("ない") || root.ends_with("なかった") {
            // dict-grammar.lisp:528-529 (find-word-with-conj-prop root
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
        score: ScoreMod::Constant(200),
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

    /// `:desu` cache `kf` for "です", REPL pinned: id=71736, seq=1628500,
    /// text="です", ord=0, common=0, common_tags="[spec1]",
    /// conjugate_p=T, nokanji=NIL, best_kanji=:NULL.
    fn kf_desu() -> KanaText {
        KanaText {
            id: 71736,
            seq: 1628500,
            text: "です".into(),
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

    /// REPL: `(suffix-desu "食べない" "です" kf)` → 1 COMPOUND
    /// text="食べないです" kana="たべない です" score-mod=(constantly 200)
    /// connector=" " primary=KANJI-TEXT (食べない id=411231 seq=10092227).
    /// "ない"-tail branch into conj-neg filter.
    #[tokio::test]
    async fn desu1_nai_tail_kanji() {
        let ctx = ctx().await;
        let kf = kf_desu();
        let result = suffix_desu(&ctx, "食べない", "です", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べないです");
        assert_eq!(c.kana, "たべない です");
        assert!(matches!(c.score_mod, ScoreMod::Constant(200)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 411231);
                assert_eq!(k.seq, 10092227);
                assert_eq!(k.text, "食べない");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-desu "ない" "です" kf)` → 3 COMPOUNDs text="ないです"
    /// kana="ない です" each — find-word-with-conj-prop on bare "ない"
    /// yields three kana-text rows (seqs 2257550 / 10320151 / 10470712).
    #[tokio::test]
    async fn desu2_bare_nai_kana() {
        let ctx = ctx().await;
        let kf = kf_desu();
        let result = suffix_desu(&ctx, "ない", "です", &kf).await.unwrap();
        assert_eq!(result.len(), 3);
        for c in &result {
            assert_eq!(c.text, "ないです");
            assert_eq!(c.kana, "ない です");
            assert!(matches!(c.score_mod, ScoreMod::Constant(200)));
            assert!(c.score_base.is_none());
            assert_eq!(c.words.len(), 2);
        }
        let seqs: std::collections::HashSet<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => -1,
            })
            .collect();
        assert!(seqs.contains(&2257550));
        assert!(seqs.contains(&10320151));
        assert!(seqs.contains(&10470712));
    }

    /// REPL: `(suffix-desu "行かなかった" "です" kf)` → 1 COMPOUND
    /// text="行かなかったです" kana="いかなかった です"
    /// primary=KANJI-TEXT (行かなかった id=922673 seq=10349396). Exercises
    /// the "なかった"-tail branch.
    #[tokio::test]
    async fn desu3_nakatta_tail() {
        let ctx = ctx().await;
        let kf = kf_desu();
        let result = suffix_desu(&ctx, "行かなかった", "です", &kf)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "行かなかったです");
        assert_eq!(c.kana, "いかなかった です");
        assert!(matches!(c.score_mod, ScoreMod::Constant(200)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 922673);
                assert_eq!(k.seq, 10349396);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-desu "じゃない" "です" kf)` → 1 COMPOUND
    /// text="じゃないです" kana="じゃない です" primary=KANA-TEXT
    /// (じゃない id=3289329 seq=10019714 ord=1). Pins a じゃない-tail
    /// case that still goes through the "ない" check.
    #[tokio::test]
    async fn desu4_janai_tail() {
        let ctx = ctx().await;
        let kf = kf_desu();
        let result = suffix_desu(&ctx, "じゃない", "です", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "じゃないです");
        assert_eq!(c.kana, "じゃない です");
        assert!(matches!(c.score_mod, ScoreMod::Constant(200)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.id, 3289329);
                assert_eq!(k.seq, 10019714);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }

    /// REPL: each of `"食べ"`, `"行きません"`, `"ありません"` → NIL —
    /// none end with "ない" / "なかった", so the outer `and` short-circuits.
    #[tokio::test]
    async fn desu5_no_nai_suffix_returns_empty() {
        let ctx = ctx().await;
        let kf = kf_desu();
        for r in ["食べ", "行きません", "ありません"] {
            let result = suffix_desu(&ctx, r, "です", &kf).await.unwrap();
            assert!(result.is_empty(), "expected NIL for root={:?}", r);
        }
    }
}
