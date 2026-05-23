//! Port of `ichiran/dict:suffix-sa` (`dict-grammar.lisp:481`).
//!
//! ```lisp
//! (def-simple-suffix suffix-sa :sa (:connector "" :score 2) (root)
//!   (nconc
//!    (find-word-with-conj-type root +conj-adjective-stem+)
//!    (find-word-with-pos root "adj-na")))
//! ```
//!
//! `+conj-adjective-stem+` is `51` (`dict-errata.lisp:1237`); the
//! `&rest` conj-type set is `(51)`. `nconc` concatenates the adj-i
//! stem rows (arm A) before the `adj-na` rows (arm B). Mapcar tail
//! delegated to [`def_simple_suffix_body`].
//!
//! `suf` typed `&KanaText`: the `:sa` cache loads kana-texts via
//! `(load-kf :sa (get-kana-form 2029120 "さ"))`.

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::find_word_with_pos::{find_word_with_pos, WordWithPosRows};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;

pub async fn suffix_sa(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:483 — (find-word-with-conj-type root +conj-adjective-stem+)
    // dict-errata.lisp:1237 — (defconstant +conj-adjective-stem+ 51)
    let mut primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[51])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:484 — (find-word-with-pos root "adj-na")
    let adj_na = match find_word_with_pos(ctx, root, &["adj-na"]).await? {
        WordWithPosRows::Kana(rows) => rows
            .into_iter()
            .map(|r| PrimaryWord::from(KaniWordDispatchEnum::Kana(r)))
            .collect::<Vec<_>>(),
        WordWithPosRows::Kanji(rows) => rows
            .into_iter()
            .map(|r| PrimaryWord::from(KaniWordDispatchEnum::Kanji(r)))
            .collect::<Vec<_>>(),
    };
    // dict-grammar.lisp:482 — (nconc arm-A arm-B)
    primary_words.extend(adj_na);

    // dict-grammar.lisp:481 — (:connector "" :score 2), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(2),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suf, kf, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::simple_text_class::SimpleText;

    /// `:sa` suffix-cache kf for "さ", REPL pinned via
    /// `(postmodern:get-dao 'kana-text 110392)`: id=110392, seq=2029120,
    /// text="さ", ord=0, common=0, common_tags="[spec1]",
    /// conjugate_p=T, nokanji=NIL, best_kanji=:NULL.
    fn kf_sa() -> KanaText {
        KanaText {
            id: 110392,
            seq: 2029120,
            text: "さ".into(),
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

    /// REPL SA1: `(suffix-sa "美し" "さ" kf-sa)` → 1 COMPOUND
    /// text="美しさ" kana="うつくしさ" score-mod=2 score-base=NIL
    /// primary=KANJI-TEXT (美し id=263320 seq=10017294),
    /// words=(primary, kf-sa). Exercises arm A (conj-type 51) only.
    #[tokio::test]
    async fn sa1_adj_i_stem_kanji() {
        let ctx = ctx().await;
        let kf = kf_sa();
        let result = suffix_sa(&ctx, "美し", "さ", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "美しさ");
        assert_eq!(c.kana, "うつくしさ");
        assert!(matches!(c.score_mod, ScoreMod::Single(2)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 263320);
                assert_eq!(k.seq, 10017294);
                assert_eq!(k.text, "美し");
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

    /// REPL SA2: `(suffix-sa "静か" "さ" kf-sa)` → 1 COMPOUND
    /// text="静かさ" kana="しずかさ" score-mod=2 score-base=NIL
    /// primary=KANJI-TEXT (静か id=31238 seq=1381820),
    /// words=(primary, kf-sa). Exercises arm B (adj-na) only.
    #[tokio::test]
    async fn sa2_adj_na_kanji() {
        let ctx = ctx().await;
        let kf = kf_sa();
        let result = suffix_sa(&ctx, "静か", "さ", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "静かさ");
        assert_eq!(c.kana, "しずかさ");
        assert!(matches!(c.score_mod, ScoreMod::Single(2)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 31238);
                assert_eq!(k.seq, 1381820);
                assert_eq!(k.text, "静か");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        // adjoin_word puts word1 at words[0] (dict.lisp:644 — `(list word1 word2)`).
        assert_eq!(c.words.len(), 2);
        match &c.words[0] {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.id, 31238),
            other => panic!("expected Kanji words[0] (primary), got {:?}", other),
        }
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL SA3: `(suffix-sa "やわらか" "さ" kf-sa)` → 2 COMPOUNDs
    /// (one from each arm, both KANA-TEXT). Arm A: id=1018986
    /// seq=10639355. Arm B: id=53460 seq=1460730. Both text="やわらか",
    /// kana="やわらかさ". Exercises the nconc concatenation order
    /// (arm A before arm B).
    #[tokio::test]
    async fn sa3_both_arms_kana_yawaraka() {
        let ctx = ctx().await;
        let kf = kf_sa();
        let result = suffix_sa(&ctx, "やわらか", "さ", &kf).await.unwrap();
        assert_eq!(result.len(), 2);
        for c in &result {
            assert_eq!(c.text, "やわらかさ");
            assert_eq!(c.kana, "やわらかさ");
            assert!(matches!(c.score_mod, ScoreMod::Single(2)));
            assert!(c.score_base.is_none());
            match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => assert_eq!(k.text, "やわらか"),
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
        // nconc order: arm-A (conj-type 51) first, arm-B (adj-na) second.
        let ids: Vec<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.id,
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(ids, vec![1018986, 53460]);
        let seqs: Vec<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(seqs, vec![10639355, 1460730]);
    }

    /// REPL SA4: `(suffix-sa "食べる" "さ" kf-sa)` → NIL. 食べる is a
    /// verb, neither an adj-i stem (conj-type 51) nor an adj-na noun.
    #[tokio::test]
    async fn sa4_no_match_verb() {
        let ctx = ctx().await;
        let kf = kf_sa();
        let result = suffix_sa(&ctx, "食べる", "さ", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL SA5: `(suffix-sa "高" "さ" kf-sa)` → 1 COMPOUND
    /// text="高さ" kana="たかさ" score-mod=2 score-base=NIL
    /// primary=KANJI-TEXT (高 id=1422119 seq=10591797),
    /// words=(primary, kf-sa). Exercises arm A on a single-char kanji
    /// stem.
    #[tokio::test]
    async fn sa5_adj_i_stem_single_kanji() {
        let ctx = ctx().await;
        let kf = kf_sa();
        let result = suffix_sa(&ctx, "高", "さ", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "高さ");
        assert_eq!(c.kana, "たかさ");
        assert!(matches!(c.score_mod, ScoreMod::Single(2)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 1422119);
                assert_eq!(k.seq, 10591797);
                assert_eq!(k.text, "高");
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
}
