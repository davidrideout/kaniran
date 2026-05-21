//! Port of `ichiran/dict:suffix-suru` (`dict-grammar.lisp:432`).
//!
//! ```lisp
//! (def-simple-suffix suffix-suru :suru (:connector " " :score 5) (root)
//!   (find-word-with-pos root "vs"))
//! ```
//!
//! Macro expands via `def-simple-suffix` (`dict-grammar.lisp:340-368`)
//! + `defsuffix` (`dict-grammar.lisp:332-336`) into a 3-arg fn that
//! `mapcar`s [`adjoin_word`] over the body's primary words.
//!
//! Divergences from `(root sv suf)`:
//! - `suf` typed `&KanaText` (the suffix-cache `kf` is always a
//!   kana-text under `(load-conjs :suru …)`), wrapped at the
//!   `adjoin_word` callsite.
//! - The macro's `(when (listp pw) …)` `score-base` branch is dead
//!   here (`find-word-with-pos` returns bare rows) and omitted.

use crate::conn::kani_context::KaniranContext;
use crate::dict::adjoin_word::adjoin_word;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::find_word_with_pos::{find_word_with_pos, WordWithPosRows};
use crate::dict::get_kana::get_kana;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};

pub async fn suffix_suru(
    ctx: &KaniranContext,
    root: &str,
    sv: &str,
    suf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:433 — (find-word-with-pos root "vs")
    let primary_words: Vec<KaniWordDispatchEnum> =
        match find_word_with_pos(ctx, root, &["vs"]).await? {
            WordWithPosRows::Kana(rows) => rows
                .into_iter()
                .map(KaniWordDispatchEnum::Kana)
                .collect(),
            WordWithPosRows::Kanji(rows) => rows
                .into_iter()
                .map(KaniWordDispatchEnum::Kanji)
                .collect(),
        };

    let mut out = Vec::with_capacity(primary_words.len());
    for pw in primary_words {
        // dict-grammar.lisp:357 ((destem k 0) leaves k unchanged; nil → "")
        let pw_kana = get_kana(ctx, &pw).await?.unwrap_or_default();
        let text = format!("{}{}", root, sv);
        // dict-grammar.lisp:364 (:connector " ")
        let kana = format!("{}{}{}", pw_kana, " ", sv);
        let compound = adjoin_word(
            ctx,
            pw,
            KaniSimpleTextDispatchEnum::Kana(suf.clone()),
            Some(text),
            Some(kana),
            // dict-grammar.lisp:366 — :score-mod 5
            Some(ScoreMod::Single(5)),
            None,
        )
        .await?;
        out.push(compound);
    }
    Ok(out)
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
    /// text="ジョギングし" kana="ジョギング し" — exercises the
    /// kana-text dispatch arm of `find-word-with-pos` (pure-katakana
    /// input).
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
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "ジョギング");
                assert_eq!(k.seq, 1066360);
            }
            other => panic!("expected Kana primary, got {:?}", other),
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
