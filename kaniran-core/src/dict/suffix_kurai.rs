//! Port of `ichiran/dict:suffix-kurai` (`dict-grammar.lisp:540`).
//!
//! ```lisp
//! (def-simple-suffix suffix-kurai :kurai (:connector " " :score 3) (root)
//!   (find-word-with-conj-type root 2))
//! ```
//!
//! Mapcar tail delegated to [`def_simple_suffix_body`]. Conj-type 2 is
//! past-plain (た-form).
//!
//! Divergences from `(root sv suf)`:
//! - `suf` typed `&KanaText` (the `:kurai` cache rows are loaded by
//!   `(load-kf :kurai (get-kana-form 1154340 "くらい"))` /
//!   `(load-kf :kurai (get-kana-form 1154340 "ぐらい"))` — both
//!   materialize kana-texts).

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_kurai(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:541 — (find-word-with-conj-type root 2)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[2])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:540 — (:connector " " :score 3), :stem 0 default.
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
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::simple_text_class::SimpleText;

    /// `:kurai` suffix-cache `kf` for "くらい", REPL pinned: `(get-kana-
    /// form 1154340 "くらい")` → id=21985, seq=1154340, text="くらい",
    /// common=0, common_tags="[spec1]", conjugate_p=T, nokanji=nil,
    /// best_kanji=:NULL.
    fn kf_kurai() -> KanaText {
        KanaText {
            id: 21985,
            seq: 1154340,
            text: "くらい".into(),
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

    /// REPL KURAI1: `(suffix-kurai "食べた" "くらい" kf-kurai)` → 1
    /// COMPOUND text="食べたくらい" kana="たべた くらい" score-mod=3
    /// score-base=NIL primary=KANJI-TEXT (食べた seq 10092229),
    /// words=(primary kf). Note the space in kana from connector=" ".
    #[tokio::test]
    async fn kurai1_tabeta_kurai_kanji() {
        let ctx = ctx().await;
        let kf = kf_kurai();
        let result = suffix_kurai(&ctx, "食べた", "くらい", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べたくらい");
        assert_eq!(c.kana, "たべた くらい");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
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

    /// REPL KURAI2: `(suffix-kurai "見た" "くらい" kf-kurai)` → 1
    /// COMPOUND text="見たくらい" kana="みた くらい" score-mod=3
    /// primary=KANJI-TEXT (見た seq 10315009).
    #[tokio::test]
    async fn kurai2_mita_kurai() {
        let ctx = ctx().await;
        let kf = kf_kurai();
        let result = suffix_kurai(&ctx, "見た", "くらい", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "見たくらい");
        assert_eq!(c.kana, "みた くらい");
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "見た");
                assert_eq!(k.seq, 10315009);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL KURAI3: `(suffix-kurai "した" "くらい" kf-kurai)` → 1
    /// COMPOUND text="したくらい" kana="した くらい" primary=KANA-TEXT
    /// (した seq 10152246). Exercises the kana-text arm.
    #[tokio::test]
    async fn kurai3_shita_kana() {
        let ctx = ctx().await;
        let kf = kf_kurai();
        let result = suffix_kurai(&ctx, "した", "くらい", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "したくらい");
        assert_eq!(c.kana, "した くらい");
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "した");
                assert_eq!(k.seq, 10152246);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }

    /// REPL KURAI4: `(suffix-kurai "無理" "くらい" kf-kurai)` → NIL.
    /// 無理 has no conj-type-2 row.
    #[tokio::test]
    async fn kurai4_non_verb_root() {
        let ctx = ctx().await;
        let kf = kf_kurai();
        let result = suffix_kurai(&ctx, "無理", "くらい", &kf).await.unwrap();
        assert!(result.is_empty());
    }
}
