//! Port of `ichiran/dict:word-info-from-text` (`dict.lisp:1382`).
//!
//! Builds a one-span segment-list over `text` (looking up its full
//! readings and scoring each) and collapses it into a single
//! [`WordInfo`] via [`word_info_from_segment_list`].

use crate::conn::kani_context::KaniranContext;
use crate::dict::find_word_full::{find_word_full, CounterArg};
use crate::dict::gen_score::gen_score;
use crate::dict::segment_list_struct::SegmentList;
use crate::dict::segment_struct::Segment;
use crate::dict::word_info_class::WordInfo;
use crate::dict::word_info_from_segment_list::word_info_from_segment_list;

pub async fn word_info_from_text(
    ctx: &KaniranContext,
    text: &str,
) -> Result<WordInfo, sqlx::Error> {
    // dict.lisp:1384 (readings (find-word-full text :counter :auto))
    let readings = find_word_full(ctx, text, false, Some(CounterArg::Auto)).await?;
    // dict.lisp:1385 (segments (loop for r in readings collect (gen-score (make-segment …))))
    let text_len = text.chars().count();
    let mut segments: Vec<Segment> = Vec::with_capacity(readings.len());
    for r in readings {
        let mut segment = Segment {
            start: 0,
            end: text_len,
            word: r,
            score: None,
            info: None,
            top: None,
            text: Some(text.to_string()),
        };
        gen_score(ctx, &mut segment, false, &[]).await?;
        segments.push(segment);
    }
    // dict.lisp:1386-1387 (segment-list (make-segment-list :segments segments :start 0 :end (length text) :matches (length segments)))
    let matches = segments.len();
    let mut segment_list = SegmentList {
        segments,
        start: 0,
        end: text_len,
        top: None,
        matches,
    };
    // dict.lisp:1388 (word-info-from-segment-list segment-list)
    word_info_from_segment_list(ctx, &mut segment_list).await
}

#[cfg(test)]
mod tests {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //!
    //! Every case is a single-survivor result (`skipped = 0`, i.e.
    //! `find-word-full` returned exactly one reading), so the outcome is
    //! independent of `find-word`'s unordered row order. Multi-survivor
    //! inputs (where `wi1` is order-dependent) are exercised
    //! deterministically by `word_info_from_segment_list`'s own tests.
    use super::*;
    use crate::dict::word_info_class::{WordInfoKana, WordInfoSeq, WordInfoType};

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL (.103, word-info-from-text "図書館"): single KANJI reading.
    #[tokio::test]
    async fn simple_kanji_noun() {
        let ctx = ctx_from_env().await;
        let wi = word_info_from_text(&ctx, "図書館").await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kanji);
        assert_eq!(wi.text, "図書館");
        assert_eq!(wi.kana, Some(WordInfoKana::Single("としょかん".into())));
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(1370420)));
        assert_eq!(wi.score, Some(952));
        assert!(!wi.alternative);
        assert_eq!(wi.skipped, 0);
        assert_eq!(wi.start, Some(0));
        assert_eq!(wi.end, Some(3));
        assert!(wi.components.is_empty());
        assert!(wi.counter.is_none());
        assert_eq!(wi.true_text.as_deref(), Some("図書館"));
        assert!(wi.conjugations.is_none());
        assert!(wi.primary);
    }

    /// REPL (.103, word-info-from-text "オレら"): single KANA reading
    /// (seq 1576880). `end = 3` confirms character (not byte) length.
    #[tokio::test]
    async fn simple_kana_pronoun() {
        let ctx = ctx_from_env().await;
        let wi = word_info_from_text(&ctx, "オレら").await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kana);
        assert_eq!(wi.text, "オレら");
        assert_eq!(wi.kana, Some(WordInfoKana::Single("オレら".into())));
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(1576880)));
        assert_eq!(wi.score, Some(24));
        assert!(!wi.alternative);
        assert_eq!(wi.end, Some(3));
        assert!(wi.components.is_empty());
        assert_eq!(wi.true_text.as_deref(), Some("オレら"));
        assert!(wi.conjugations.is_none());
        assert!(wi.primary);
    }

    /// REPL (.103, word-info-from-text "食べてる"): single COMPOUND
    /// (食べて + いる) — the suffix-teiru expansion. seq is the
    /// per-child list; two components carry the part readings.
    #[tokio::test]
    async fn compound_teiru() {
        let ctx = ctx_from_env().await;
        let wi = word_info_from_text(&ctx, "食べてる").await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kanji);
        assert_eq!(wi.text, "食べてる");
        assert_eq!(wi.kana, Some(WordInfoKana::Single("たべてる".into())));
        assert_eq!(
            wi.seq,
            Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(10092233)),
                Some(WordInfoSeq::Single(1577980)),
            ]))
        );
        assert_eq!(wi.score, Some(434));
        assert_eq!(wi.end, Some(4));
        assert_eq!(wi.components.len(), 2);
        assert_eq!(wi.components[0].text, "食べて");
        assert_eq!(wi.components[0].seq, Some(WordInfoSeq::Single(10092233)));
        assert_eq!(wi.components[1].text, "いる");
        assert_eq!(wi.components[1].seq, Some(WordInfoSeq::Single(1577980)));
        // compound-text is not simple-text → true-text / conjugations nil.
        assert!(wi.true_text.is_none());
        assert!(wi.conjugations.is_none());
        assert!(wi.primary);
    }

    /// REPL (.103, word-info-from-text "5万100"): the `:counter :auto`
    /// branch resolves a COUNTER reading — counter pair populated,
    /// `seq` nil. `end = 5` is the character count (byte length is 7).
    #[tokio::test]
    async fn counter_auto_number() {
        let ctx = ctx_from_env().await;
        let wi = word_info_from_text(&ctx, "5万100").await.unwrap();
        assert_eq!(wi.text, "5万100");
        assert_eq!(wi.counter, Some(("Value: 50100".into(), false)));
        assert_eq!(wi.seq, None);
        assert_eq!(wi.score, Some(780));
        assert_eq!(wi.end, Some(5));
        assert!(wi.components.is_empty());
        // counter-text is not simple-text → true-text / conjugations nil.
        assert!(wi.true_text.is_none());
        assert!(wi.conjugations.is_none());
        assert!(wi.primary);
    }

    /// REPL (.103, word-info-from-text "三羽"): `:counter :auto` yields
    /// the 三羽 counter reading (value 3).
    #[tokio::test]
    async fn counter_auto_kanji_number() {
        let ctx = ctx_from_env().await;
        let wi = word_info_from_text(&ctx, "三羽").await.unwrap();
        assert_eq!(wi.text, "三羽");
        assert_eq!(wi.kana, Some(WordInfoKana::Single("さんば".into())));
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(1607310)));
        assert_eq!(wi.counter, Some(("Value: 3".into(), false)));
        assert_eq!(wi.score, Some(286));
        assert_eq!(wi.end, Some(2));
        // counter-text is not simple-text → true-text / conjugations nil.
        assert!(wi.true_text.is_none());
        assert!(wi.conjugations.is_none());
        assert!(wi.primary);
    }
}
