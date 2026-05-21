//! Port of `ichiran/dict:suffix-ra` (`dict-grammar.lisp:504`).
//!
//! ```lisp
//! (def-simple-suffix suffix-ra :ra (:connector "" :score 1) (root)
//!   (unless (alexandria:ends-with-subseq "ら" root)
//!     (or (or-as-hiragana 'find-word-with-pos root "pn")
//!         (find-word-seq root 1580640))))
//! ```
//!
//! Macro expands via `def-simple-suffix` (`dict-grammar.lisp:340-368`)
//! + `defsuffix` (`dict-grammar.lisp:332-336`) into a 3-arg fn that
//! `mapcar`s [`adjoin_word`] over the body's primary words.
//!
//! Divergences from `(root sv suf)`:
//! - `suf` typed `&KanaText` (the suffix-cache `kf` is the single
//!   `(load-kf :ra (get-kana-form 2067770 "ら"))` row), wrapped at
//!   the `adjoin_word` callsite.
//! - The macro's `(when (listp pw) …)` `score-base` branch is dead
//!   here (all three lookup paths return bare rows) and omitted.

use crate::conn::kani_context::KaniranContext;
use crate::dict::adjoin_word::adjoin_word;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::find_word::FindWordRows;
use crate::dict::find_word_seq::{find_word_seq, WordSeqRows};
use crate::dict::find_word_with_pos::{find_word_with_pos, WordWithPosRows};
use crate::dict::get_kana::get_kana;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::or_as_hiragana::{or_as_hiragana, OrAsHiraganaFinder, OrAsHiraganaRows};
use std::sync::Arc;

pub async fn suffix_ra(
    ctx: &KaniranContext,
    root: &str,
    sv: &str,
    suf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:505 — (unless (alexandria:ends-with-subseq "ら" root) …)
    if root.ends_with('ら') {
        return Ok(Vec::new());
    }

    // dict-grammar.lisp:506 — (or-as-hiragana 'find-word-with-pos root "pn")
    let finder: OrAsHiraganaFinder<'_> = Arc::new(|word: String| {
        Box::pin(async move {
            let rows = find_word_with_pos(ctx, &word, &["pn"]).await?;
            Ok(match rows {
                WordWithPosRows::Kana(v) => FindWordRows::Kana(v),
                WordWithPosRows::Kanji(v) => FindWordRows::Kanji(v),
            })
        })
    });
    let from_pn: Option<Vec<KaniWordDispatchEnum>> =
        or_as_hiragana(ctx, root, finder).await?.map(|rows| match rows {
            OrAsHiraganaRows::Direct(FindWordRows::Kana(v)) => {
                v.into_iter().map(KaniWordDispatchEnum::Kana).collect()
            }
            OrAsHiraganaRows::Direct(FindWordRows::Kanji(v)) => {
                v.into_iter().map(KaniWordDispatchEnum::Kanji).collect()
            }
            OrAsHiraganaRows::AsHiragana(v) => {
                v.into_iter().map(KaniWordDispatchEnum::Proxy).collect()
            }
        });

    // dict-grammar.lisp:507 — (find-word-seq root 1580640).
    // `or` falls through to this when `or-as-hiragana` returned nil.
    let primary_words: Vec<KaniWordDispatchEnum> = match from_pn {
        Some(words) => words,
        None => match find_word_seq(ctx, root, &[1580640]).await? {
            WordSeqRows::Kana(rows) => rows
                .into_iter()
                .map(KaniWordDispatchEnum::Kana)
                .collect(),
            WordSeqRows::Kanji(rows) => rows
                .into_iter()
                .map(KaniWordDispatchEnum::Kanji)
                .collect(),
        },
    };

    let mut out = Vec::with_capacity(primary_words.len());
    for pw in primary_words {
        // dict-grammar.lisp:357 ((destem k 0) leaves k unchanged; nil → "")
        let pw_kana = get_kana(ctx, &pw).await?.unwrap_or_default();
        let text = format!("{}{}", root, sv);
        // dict-grammar.lisp:504 (:connector "")
        let kana = format!("{}{}", pw_kana, sv);
        let compound = adjoin_word(
            ctx,
            pw,
            KaniSimpleTextDispatchEnum::Kana(suf.clone()),
            Some(text),
            Some(kana),
            // dict-grammar.lisp:504 — :score 1
            Some(ScoreMod::Single(1)),
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

    /// `:ra` suffix-cache `kf`, REPL pinned: `(get-kana-form 2067770
    /// "ら")` → id=114553, seq=2067770, text="ら", common=:NULL,
    /// common_tags="[spec1]", conjugate_p=T, nokanji=nil,
    /// best_kanji="等", hintedp=nil. Pulled verbatim from
    /// `corpus/extracted_chunk_c_suffix_abbr_2026_05_16/dict/\
    /// suffix_ra.parquet` row 3750.
    fn kf_ra() -> KanaText {
        KanaText {
            id: 114553,
            seq: 2067770,
            text: "ら".into(),
            ord: 0,
            common: None,
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: Some("等".into()),
            state: SimpleText::default(),
        }
    }

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL RA1: `(suffix-ra "我々" "ら" kf-ra)` → 1 COMPOUND
    /// text="我々ら" kana="われわれら" score-mod=1 primary=KANJI-TEXT
    /// (我々 seq 1607050).
    #[tokio::test]
    async fn ra1_kanji_pn_match() {
        let ctx = ctx().await;
        let kf = kf_ra();
        let result = suffix_ra(&ctx, "我々", "ら", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "我々ら");
        assert_eq!(c.kana, "われわれら");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "我々");
                assert_eq!(k.seq, 1607050);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
    }

    /// REPL RA2: `(suffix-ra "ばら" "ら" kf-ra)` → 0 (the UNLESS
    /// branch fires for any root ending in ら).
    #[tokio::test]
    async fn ra2_root_ends_with_ra() {
        let ctx = ctx().await;
        let kf = kf_ra();
        let result = suffix_ra(&ctx, "ばら", "ら", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL RA3: `(suffix-ra "私" "ら" kf-ra)` → 13 COMPOUNDs (one
    /// per kanji-text row of 私 with a `pn` sense). Exercises the
    /// polysemy + multi-kana branch (`get-kana` returns a different
    /// best_kana per row → distinct compound `kana` values).
    #[tokio::test]
    async fn ra3_kanji_pn_polysemy_thirteen_compounds() {
        let ctx = ctx().await;
        let kf = kf_ra();
        let result = suffix_ra(&ctx, "私", "ら", &kf).await.unwrap();
        assert_eq!(result.len(), 13);
        // Every compound shares text="私ら" and primary text="私".
        for c in &result {
            assert_eq!(c.text, "私ら");
            assert!(matches!(c.score_mod, ScoreMod::Single(1)));
            match &*c.primary {
                KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.text, "私"),
                other => panic!("expected Kanji primary, got {:?}", other),
            }
        }
        // REPL-pinned: 13 distinct compound kanas (one per best-kana
        // of the 13 私 rows). The set is order-independent (SBCL
        // SQL row iteration is unspecified) — compare as sorted set.
        let mut got: Vec<String> = result.iter().map(|c| c.kana.clone()).collect();
        got.sort();
        let mut expected: Vec<String> = vec![
            "わたしら", "あたしら", "わらわら", "わしら", "あっしら", "わいら",
            "わたいら", "わたくしら", "わっちら", "わてら", "あたいら", "あてら",
            "わちきら",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        expected.sort();
        assert_eq!(got, expected);
    }

    /// REPL RA4: `(suffix-ra "青空" "ら" kf-ra)` → 0 (青空 has no
    /// `pn` sense and 1580640 is the seq of 等, not 青空).
    #[tokio::test]
    async fn ra4_kanji_no_pn_no_fallback_seq() {
        let ctx = ctx().await;
        let kf = kf_ra();
        let result = suffix_ra(&ctx, "青空", "ら", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL RA5: `(suffix-ra "等" "ら" kf-ra)` → 0. The seq 1580640
    /// is reserved for the find-word-seq fallback path; 等 does not
    /// hit `pn` so `or-as-hiragana` falls through, and 等 itself isn't
    /// a kanji_text row at that seq, so find-word-seq also misses.
    #[tokio::test]
    async fn ra5_etc_kanji_no_match() {
        let ctx = ctx().await;
        let kf = kf_ra();
        let result = suffix_ra(&ctx, "等", "ら", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL RA6: `(suffix-ra "アナタ" "ら" kf-ra)` → 2 COMPOUNDs
    /// where each primary is a PROXY-TEXT wrapping the hiragana
    /// kana_text あなた (`or-as-hiragana` fallback path). The
    /// compound's `kana` is built off the proxy's `kana` slot, which
    /// carries the katakana surface form — so `kana = "アナタら"`.
    #[tokio::test]
    async fn ra6_katakana_as_hiragana_proxy_fallback() {
        let ctx = ctx().await;
        let kf = kf_ra();
        let result = suffix_ra(&ctx, "アナタ", "ら", &kf).await.unwrap();
        assert_eq!(result.len(), 2);
        for c in &result {
            assert_eq!(c.text, "アナタら");
            assert_eq!(c.kana, "アナタら");
            assert!(matches!(c.score_mod, ScoreMod::Single(1)));
            match &*c.primary {
                KaniWordDispatchEnum::Proxy(p) => assert_eq!(p.text, "アナタ"),
                other => panic!("expected Proxy primary, got {:?}", other),
            }
        }
    }

    /// REPL RA7: `(suffix-ra "わたし" "ら" kf-ra)` → 1 COMPOUND
    /// text="わたしら" kana="わたしら" primary=KANA-TEXT (わたし seq
    /// 1311110). Exercises the hiragana-direct `kana_text` arm of
    /// `find-word-with-pos`.
    #[tokio::test]
    async fn ra7_hiragana_direct_kana_text() {
        let ctx = ctx().await;
        let kf = kf_ra();
        let result = suffix_ra(&ctx, "わたし", "ら", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "わたしら");
        assert_eq!(c.kana, "わたしら");
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "わたし");
                assert_eq!(k.seq, 1311110);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }
}
