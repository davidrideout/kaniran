//! Port of `ichiran/dict:suffix-suru` (`dict-grammar.lisp:432`).
//!
//! ```lisp
//! (def-simple-suffix suffix-suru :suru (:connector " " :score 5) (root)
//!   (find-word-with-pos root "vs"))
//! ```
//!
//! Mapcar tail delegated to [`def_simple_suffix_body`] per CONVENTIONS
//! §4.6 case (c).
//!
//! Divergences from `(root sv suf)`:
//! - `suf` typed `&KanaText` (the suffix-cache `kf` is always a
//!   kana-text under `(load-conjs :suru …)`).

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_pos::{find_word_with_pos, WordWithPosRows};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;

pub async fn suffix_suru(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:433 — (find-word-with-pos root "vs")
    let primary_words: Vec<PrimaryWord> = match find_word_with_pos(ctx, root, &["vs"]).await? {
        WordWithPosRows::Kana(rows) => rows
            .into_iter()
            .map(|r| PrimaryWord::from(KaniWordDispatchEnum::Kana(r)))
            .collect(),
        WordWithPosRows::Kanji(rows) => rows
            .into_iter()
            .map(|r| PrimaryWord::from(KaniWordDispatchEnum::Kanji(r)))
            .collect(),
    };

    // dict-grammar.lisp:432 — (:connector " " :score 5), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(5),
        connector: " ",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::simple_text_class::SimpleText;

    /// Construct the `:suru` suffix-cache `kf` REPL pinned for the
    /// test corpus: id=439727, seq=10152292, text="し",
    /// conjugate_p=nil, nokanji=nil, best_kanji=:NULL, conjugations
    /// referencing seq 153220, hintedp=nil. Pulled verbatim from
    /// `corpus/extracted_chunk_c_suffix_abbr_2026_05_16/dict/\
    /// suffix_suru.parquet` row 0.
    fn kf_suru() -> KanaText {
        KanaText {
            id: 439727,
            seq: 10152292,
            text: "し".into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
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

    /// REPL T1: `(suffix-suru "区別" "し" kf-suru)` → 1 COMPOUND
    /// text="区別し" kana="くべつ し" score-mod=5 primary=KANJI-TEXT
    /// (区別 seq 1244250).
    #[tokio::test]
    async fn t1_kanji_root_with_vs_pos() {
        let ctx = ctx().await;
        let kf = kf_suru();
        let result = suffix_suru(&ctx, "区別", "し", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "区別し");
        assert_eq!(c.kana, "くべつ し");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "区別");
                assert_eq!(k.seq, 1244250);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
    }

    /// REPL T2: `(suffix-suru "青空" "し" kf-suru)` → 0 (青空 has no
    /// `vs` pos in `sense_prop`).
    #[tokio::test]
    async fn t2_kanji_root_no_vs_match() {
        let ctx = ctx().await;
        let kf = kf_suru();
        let result = suffix_suru(&ctx, "青空", "し", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL T3: `(suffix-suru "ジョギング" "し" kf-suru)` → 1 COMPOUND
    /// text="ジョギングし" kana="ジョギング し" score-mod=5
    /// score-base=NIL primary=KANA-TEXT (ジョギング seq 1066360),
    /// words=(primary kf). Exercises the kana-text dispatch arm of
    /// `find-word-with-pos` (pure-katakana input).
    #[tokio::test]
    async fn t3_katakana_root_kana_text_arm() {
        let ctx = ctx().await;
        let kf = kf_suru();
        let result = suffix_suru(&ctx, "ジョギング", "し", &kf)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "ジョギングし");
        assert_eq!(c.kana, "ジョギング し");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "ジョギング");
                assert_eq!(k.seq, 1066360);
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

    /// REPL T4: `(suffix-suru "" "し" kf-suru)` → 0 (empty root never
    /// matches any kanji_text/kana_text row).
    #[tokio::test]
    async fn t4_empty_root() {
        let ctx = ctx().await;
        let kf = kf_suru();
        let result = suffix_suru(&ctx, "", "し", &kf).await.unwrap();
        assert!(result.is_empty());
    }
}
