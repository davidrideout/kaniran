//! Port of `ichiran/dict:suffix-rou` (`dict-grammar.lisp:461`).
//!
//! ```lisp
//! (def-simple-suffix suffix-rou :rou (:connector "" :score 1) (root)
//!   (find-word-with-conj-type root 2))
//! ```
//!
//! Mapcar tail delegated to [`def_simple_suffix_body`].
//!
//! Divergences from `(root sv suf)`:
//! - `suf` typed `&KanaText` (the `:rou` cache row is loaded by
//!   `(load-kf :rou (get-kana-form 1928670 "だろう") :text "ろう")` —
//!   a kana-text). The cache key is the `:text` override `"ろう"`; the
//!   kf object itself carries the source text `"だろう"`.

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_rou(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:462 — (find-word-with-conj-type root 2)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[2])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:461 — (:connector "" :score 1), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(1),
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

    /// `:rou` suffix-cache `kf`, REPL pinned: `(get-kana-form 1928670
    /// "だろう")` → id=99986, seq=1928670, text="だろう", common=0,
    /// common_tags="[spec1]", conjugate_p=T, nokanji=nil, best_kanji=:NULL.
    /// The cache key is the `:text` override `"ろう"`; the kf object
    /// itself carries `"だろう"`.
    fn kf_rou() -> KanaText {
        KanaText {
            id: 99986,
            seq: 1928670,
            text: "だろう".into(),
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

    /// REPL ROU1: `(suffix-rou "食べた" "ろう" kf-rou)` → 1 COMPOUND
    /// text="食べたろう" kana="たべたろう" score-mod=1 score-base=NIL
    /// primary=KANJI-TEXT (食べた seq 10092229), words=(primary kf).
    /// Exercises the past-plain (conj-type 2) kanji arm.
    #[tokio::test]
    async fn rou1_ichidan_past_kanji() {
        let ctx = ctx().await;
        let kf = kf_rou();
        let result = suffix_rou(&ctx, "食べた", "ろう", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べたろう");
        assert_eq!(c.kana, "たべたろう");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べた");
                assert_eq!(k.seq, 10092229);
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

    /// REPL ROU2: `(suffix-rou "なかった" "ろう" kf-rou)` → 4 COMPOUNDs
    /// — four conj-type-2 kana-text rows for なかった. Each has
    /// text="なかったろう" kana="なかったろう" score-mod=1 primary=KANA.
    /// Pinned seqs: 10076179, 10470716, 10517041, 10648797.
    #[tokio::test]
    async fn rou2_nakatta_polysemy_four() {
        let ctx = ctx().await;
        let kf = kf_rou();
        let result = suffix_rou(&ctx, "なかった", "ろう", &kf).await.unwrap();
        assert_eq!(result.len(), 4);
        for c in &result {
            assert_eq!(c.text, "なかったろう");
            assert_eq!(c.kana, "なかったろう");
            assert!(matches!(c.score_mod, ScoreMod::Single(1)));
            assert!(c.score_base.is_none());
            match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => assert_eq!(k.text, "なかった"),
                other => panic!("expected Kana primary, got {:?}", other),
            }
            assert_eq!(c.words.len(), 2);
        }
        let mut seqs: Vec<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => unreachable!(),
            })
            .collect();
        seqs.sort();
        assert_eq!(seqs, vec![10076179, 10470716, 10517041, 10648797]);
    }

    /// REPL ROU3: `(suffix-rou "食べる" "ろう" kf-rou)` → NIL. 食べる is
    /// a root form (conj-type :root), not past-plain (conj-type 2);
    /// find-word-with-conj-type returns 0 rows.
    #[tokio::test]
    async fn rou3_root_form_no_match() {
        let ctx = ctx().await;
        let kf = kf_rou();
        let result = suffix_rou(&ctx, "食べる", "ろう", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL ROU4: `(suffix-rou "無理" "ろう" kf-rou)` → NIL. Non-verb
    /// root.
    #[tokio::test]
    async fn rou4_non_verb_root() {
        let ctx = ctx().await;
        let kf = kf_rou();
        let result = suffix_rou(&ctx, "無理", "ろう", &kf).await.unwrap();
        assert!(result.is_empty());
    }
}
