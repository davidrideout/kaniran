//! Port of `ichiran/dict:suffix-ren` (`dict-grammar.lisp:374`).
//!
//! ```lisp
//! (def-simple-suffix suffix-ren :ren (:connector "" :score 5) (root)
//!   ;; generic ren'youkei suffix
//!   (find-word-with-conj-type root 13))
//! ```
//!
//! Mapcar tail delegated to [`def_simple_suffix_body`] per CONVENTIONS
//! §4.6 case (c).
//!
//! Divergences from `(root sv suf)`:
//! - `suf` typed `&KanaText` (the `:ren` cache rows are loaded by
//!   `(load-kf :ren …)` / `(load-conjs :ren …)` — all materialize
//!   kana-texts).

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_ren(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:376 — (find-word-with-conj-type root 13)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[13])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:374 — (:connector "" :score 5), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(5),
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

    /// `:ren` suffix-cache `kf` for つつ, REPL pinned: `(get-kana-form
    /// 1008120 "つつ")` → id=1075, seq=1008120, text="つつ", common=0,
    /// common_tags="[spec1]", conjugate_p=T, nokanji=nil,
    /// best_kanji=:NULL.
    fn kf_ren_tsutsu() -> KanaText {
        KanaText {
            id: 1075,
            seq: 1008120,
            text: "つつ".into(),
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

    /// REPL REN1: `(suffix-ren "食べ" "つつ" kf-ren-tsutsu)` → 1
    /// COMPOUND text="食べつつ" kana="たべつつ" score-mod=5
    /// primary=KANJI-TEXT (食べ seq 10092273), words=(primary kf),
    /// score-base=NIL.
    #[tokio::test]
    async fn ren1_ichidan_ren_youkei_kanji() {
        let ctx = ctx().await;
        let kf = kf_ren_tsutsu();
        let result = suffix_ren(&ctx, "食べ", "つつ", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べつつ");
        assert_eq!(c.kana, "たべつつ");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べ");
                assert_eq!(k.seq, 10092273);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        // dict.lisp:644 — (:words (list word1 word2)) — word2 is kf wrapped.
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL REN2: `(suffix-ren "無理" "つつ" kf-ren-tsutsu)` → NIL.
    /// 無理 has no conj-type-13 entry.
    #[tokio::test]
    async fn ren2_non_verb_root() {
        let ctx = ctx().await;
        let kf = kf_ren_tsutsu();
        let result = suffix_ren(&ctx, "無理", "つつ", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL REN3: `(suffix-ren "い" "つつ" kf-ren-tsutsu)` → 6
    /// COMPOUNDs (suffix-ren has NO "い" gate — six conj-type-13
    /// rows for い exist as ren'youkei stems). Each compound has
    /// text="いつつ" kana="いつつ" with a KANA-TEXT primary at
    /// text="い"; pinned seqs are 2258170, 10033674, 10128912,
    /// 10303160, 10362338, 10423311.
    #[tokio::test]
    async fn ren3_i_root_not_gated_six_rows() {
        let ctx = ctx().await;
        let kf = kf_ren_tsutsu();
        let result = suffix_ren(&ctx, "い", "つつ", &kf).await.unwrap();
        assert_eq!(result.len(), 6);
        for c in &result {
            assert_eq!(c.text, "いつつ");
            assert_eq!(c.kana, "いつつ");
            assert!(matches!(c.score_mod, ScoreMod::Single(5)));
            assert!(c.score_base.is_none());
            match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => assert_eq!(k.text, "い"),
                other => panic!("expected Kana primary, got {:?}", other),
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
        let mut seqs: Vec<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => unreachable!(),
            })
            .collect();
        seqs.sort();
        assert_eq!(
            seqs,
            vec![2258170, 10033674, 10128912, 10303160, 10362338, 10423311]
        );
    }

    /// REPL REN4: `(suffix-ren "あり" "つつ" kf-ren-tsutsu)` → 1
    /// COMPOUND text="ありつつ" kana="ありつつ" score-mod=5
    /// score-base=NIL primary=KANA-TEXT (あり seq 2150170),
    /// words=(primary kf). Exercises the kana-text arm.
    #[tokio::test]
    async fn ren4_kana_root() {
        let ctx = ctx().await;
        let kf = kf_ren_tsutsu();
        let result = suffix_ren(&ctx, "あり", "つつ", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "ありつつ");
        assert_eq!(c.kana, "ありつつ");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "あり");
                assert_eq!(k.seq, 2150170);
            }
            other => panic!("expected Kana primary, got {:?}", other),
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
}
