//! Port of `ichiran/dict:suffix-ren-` (`dict-grammar.lisp:378`).
//!
//! Score-0 ren'youkei suffix variant: looks up the root as a conj-type 13
//! conjugation (same body as `suffix-ren`, different score).

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_ren_(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:379 — (find-word-with-conj-type root 13)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[13])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:378 — (:connector "" :score 0), :stem 0 default.
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
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::simple_text_class::SimpleText;

    /// `:ren-` suffix-cache `kf` for がい, REPL pinned:
    /// `(get-kana-form 2606690 "がい")` → id=177519, seq=2606690,
    /// text="がい", common=:NULL, common_tags="", conjugate_p=T,
    /// nokanji=nil, best_kanji="甲斐".
    fn kf_ren_minus_gai() -> KanaText {
        KanaText {
            id: 177519,
            seq: 2606690,
            text: "がい".into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: Some("甲斐".into()),
            state: SimpleText::default(),
        }
    }

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL REN-1: `(suffix-ren- "食べ" "がい" kf-ren-minus-gai)` → 1
    /// COMPOUND text="食べがい" kana="たべがい" score-mod=0
    /// primary=KANJI-TEXT (食べ seq 10092273), words=(primary kf).
    #[tokio::test]
    async fn ren_minus_1_ichidan_ren_youkei_kanji() {
        let ctx = ctx().await;
        let kf = kf_ren_minus_gai();
        let result = suffix_ren_(&ctx, "食べ", "がい", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べがい");
        assert_eq!(c.kana, "たべがい");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
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

    /// REPL REN-2: `(suffix-ren- "無理" "がい" kf-ren-minus-gai)` → NIL.
    /// 無理 has no conj-type-13 entry.
    #[tokio::test]
    async fn ren_minus_2_non_verb_root() {
        let ctx = ctx().await;
        let kf = kf_ren_minus_gai();
        let result = suffix_ren_(&ctx, "無理", "がい", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL REN-3: `(suffix-ren- "い" "がい" kf-ren-minus-gai)` → 6
    /// COMPOUNDs (suffix-ren- has NO "い" gate; six conj-type-13
    /// rows exist for root "い"). Each compound has text="いがい"
    /// kana="いがい" with a KANA-TEXT primary at text="い"; pinned
    /// seqs are 2258170, 10033674, 10128912, 10303160, 10362338,
    /// 10423311.
    #[tokio::test]
    async fn ren_minus_3_i_root_not_gated_six_rows() {
        let ctx = ctx().await;
        let kf = kf_ren_minus_gai();
        let result = suffix_ren_(&ctx, "い", "がい", &kf).await.unwrap();
        assert_eq!(result.len(), 6);
        for c in &result {
            assert_eq!(c.text, "いがい");
            assert_eq!(c.kana, "いがい");
            assert!(matches!(c.score_mod, ScoreMod::Single(0)));
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
}
