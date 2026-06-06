//! Port of `ichiran/dict:suffix-rashii` (`dict-grammar.lisp:520`).
//!
//! Handles ～らしい: pairs words found as the root's past-plain (conj 2)
//! with those found as root+ら conj-type 11.

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::pair_words_by_conj::pair_words_by_conj;

pub async fn suffix_rashii(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:522 (find-word-with-conj-type root 2)
    let group_a = find_word_with_conj_type(ctx, root, &[2]).await?;
    // dict-grammar.lisp:523 (find-word-with-conj-type (concatenate root "ら") 11)
    let root_ra = format!("{}ら", root);
    let group_b = find_word_with_conj_type(ctx, &root_ra, &[11]).await?;

    // dict-grammar.lisp:521 (pair-words-by-conj group_a group_b)
    let buckets = pair_words_by_conj(ctx, &[group_a, group_b]).await?;

    // dict-grammar.lisp:351-354 (when (listp pw) (setf score-base (second pw) pw (first pw)))
    let primary_words: Vec<PrimaryWord> = buckets
        .into_iter()
        .map(|bucket| {
            let mut iter = bucket.into_iter();
            let first = iter.next().expect("pair-words-by-conj bucket missing slot 0");
            let second = iter.next().expect("pair-words-by-conj bucket missing slot 1");
            let pw = first.expect(
                "suffix-rashii: pair-words-by-conj bucket has nil at slot 0; \
                 upstream (adjoin-word nil w2) raises no-applicable-method",
            );
            match second {
                Some(score_base) => PrimaryWord::WithScoreBase(pw, score_base),
                None => PrimaryWord::Bare(pw),
            }
        })
        .collect();

    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(3),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suf, kf, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::simple_text_class::SimpleText;

    /// `:rashii` cache `kf` for "らしい", REPL pinned: id=1812,
    /// seq=1013240, text="らしい", ord=0, common=0,
    /// common_tags="[ichi1]", conjugate_p=T, nokanji=NIL,
    /// best_kanji=:NULL.
    fn kf_rashii() -> KanaText {
        KanaText {
            id: 1812,
            seq: 1013240,
            text: "らしい".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[ichi1]".into(),
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

    /// REPL: `(suffix-rashii "食べた" "らしい" kf)` → 1 COMPOUND
    /// text="食べたらしい" kana="たべたらしい" score-mod=3 primary=KANJI-TEXT
    /// (食べた id=411235 seq=10092229), score-base=KANJI-TEXT
    /// (食べたら id=411321 seq=10092265). pair-words-by-conj paired
    /// 食べた (conj-type 2) with 食べたら (conj-type 11) by shared
    /// (seq-from, via) signature.
    #[tokio::test]
    async fn rashii1_tabeta() {
        let ctx = ctx().await;
        let kf = kf_rashii();
        let result = suffix_rashii(&ctx, "食べた", "らしい", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べたらしい");
        assert_eq!(c.kana, "たべたらしい");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 411235);
                assert_eq!(k.seq, 10092229);
                assert_eq!(k.text, "食べた");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        let score_base = c.score_base.as_ref().expect("score-base must be set");
        match score_base.as_ref() {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 411321);
                assert_eq!(k.seq, 10092265);
                assert_eq!(k.text, "食べたら");
            }
            other => panic!("expected Kanji score-base (食べたら), got {:?}", other),
        }
    }

    /// REPL: `(suffix-rashii "来た" "らしい" kf)` → 1 COMPOUND
    /// text="来たらしい" kana="きたらしい" primary=KANJI-TEXT (来た
    /// id=670727 seq=10221106), score-base=KANJI-TEXT (来たら id=670813
    /// seq=10221142).
    #[tokio::test]
    async fn rashii2_kita() {
        let ctx = ctx().await;
        let kf = kf_rashii();
        let result = suffix_rashii(&ctx, "来た", "らしい", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "来たらしい");
        assert_eq!(c.kana, "きたらしい");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10221106),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        let sb = c.score_base.as_ref().expect("score-base set");
        match sb.as_ref() {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10221142),
            other => panic!("expected Kanji score-base (来たら), got {:?}", other),
        }
    }

    /// REPL: `(suffix-rashii "行った" "らしい" kf)` → 3 COMPOUNDs each
    /// text="行ったらしい". Per-bucket REPL pairings of (primary_seq,
    /// score_base_seq):
    ///   - (10402883, 10402923) — やった / やったら
    ///   - (10349394, 10349434) — いった / いったら
    ///   - (10087633, 10087672) — おこなった / おこなったら
    /// Pin each pair so pair-words-by-conj keeps its bucket signature.
    #[tokio::test]
    async fn rashii3_itta_three_readings() {
        let ctx = ctx().await;
        let kf = kf_rashii();
        let result = suffix_rashii(&ctx, "行った", "らしい", &kf).await.unwrap();
        assert_eq!(result.len(), 3);
        let expected_pairs: std::collections::HashMap<i32, i32> = [
            (10402883, 10402923),
            (10349394, 10349434),
            (10087633, 10087672),
        ]
        .into_iter()
        .collect();
        for c in &result {
            assert_eq!(c.text, "行ったらしい");
            assert!(matches!(c.score_mod, ScoreMod::Single(3)));
            assert_eq!(c.words.len(), 2);
            let primary_seq = match &*c.primary {
                KaniWordDispatchEnum::Kanji(k) => k.seq,
                other => panic!("expected Kanji primary, got {:?}", other),
            };
            let expected_score_base_seq = expected_pairs
                .get(&primary_seq)
                .unwrap_or_else(|| panic!("unexpected primary seq {}", primary_seq));
            let sb = c.score_base.as_ref().expect("score-base set");
            match sb.as_ref() {
                KaniWordDispatchEnum::Kanji(k) => assert_eq!(
                    k.seq, *expected_score_base_seq,
                    "primary {} should pair with score-base {}",
                    primary_seq, expected_score_base_seq
                ),
                other => panic!("expected Kanji score-base, got {:?}", other),
            }
        }
    }

    /// REPL: `(suffix-rashii "した" "らしい" kf)` → 1 COMPOUND
    /// text="したらしい" kana="したらしい" primary=KANA-TEXT (した
    /// id=439677 seq=10152246), score-base=KANA-TEXT (したら id=439719
    /// seq=10152284). Exercises kana-text dispatch.
    #[tokio::test]
    async fn rashii4_shita_kana() {
        let ctx = ctx().await;
        let kf = kf_rashii();
        let result = suffix_rashii(&ctx, "した", "らしい", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "したらしい");
        assert_eq!(c.kana, "したらしい");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => assert_eq!(k.seq, 10152246),
            other => panic!("expected Kana primary, got {:?}", other),
        }
        let sb = c.score_base.as_ref().expect("score-base set");
        match sb.as_ref() {
            KaniWordDispatchEnum::Kana(k) => assert_eq!(k.seq, 10152284),
            other => panic!("expected Kana score-base (したら), got {:?}", other),
        }
    }

    /// REPL: each of `"無理" "食べ"` → NIL. "無理" has no conj-type 2 row
    /// and "無理ら" has no conj-type 11; "食べ" is conj-type 13 (ren-stem)
    /// not 2, and "食べら" has no conj-type 11.
    #[tokio::test]
    async fn rashii5_no_conjugation_pair() {
        let ctx = ctx().await;
        let kf = kf_rashii();
        for r in ["無理", "食べ"] {
            let result = suffix_rashii(&ctx, r, "らしい", &kf).await.unwrap();
            assert!(result.is_empty(), "expected NIL for root={:?}", r);
        }
    }
}
