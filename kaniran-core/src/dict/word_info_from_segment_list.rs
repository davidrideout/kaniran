//! Port of `ichiran/dict:word-info-from-segment-list` (`dict.lisp:1353`).
//!
//! Maps [`word_info_from_segment`] across the captured segment-list,
//! filters scores below `2/3 * (word-info-score wi1)` (where `wi1` is
//! the FIRST pre-filter wi), then either:
//!
//! - returns `wi1` with `skipped = matches - 1` (one survivor); or
//! - builds a synthetic `word-info` carrying `wi1`'s `type` / `text` /
//!   `score`, the per-survivor `kana` / `seq` collected verbatim,
//!   `alternative = t`, and `skipped = matches - kept`.
//!
//! Diverges from the upstream lambda list `(segment-list)` by taking
//! `&KaniranContext` for the database handle (replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`]) and
//! `&mut SegmentList` so the per-segment `(get-text segment)` lazy
//! memoization (`dict.lisp:677-679`) runs in place.
//!
//! [`SEGMENT_SCORE_CUTOFF`]: crate::dict::_star_segment_score_cutoff_star_::SEGMENT_SCORE_CUTOFF

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_segment_score_cutoff_star_::SEGMENT_SCORE_CUTOFF;
use crate::dict::segment_list_struct::SegmentList;
use crate::dict::word_info_class::{WordInfo, WordInfoKana, WordInfoSeq};
use crate::dict::word_info_from_segment::word_info_from_segment;

pub async fn word_info_from_segment_list(
    ctx: &KaniranContext,
    segment_list: &mut SegmentList,
) -> Result<WordInfo, sqlx::Error> {
    // dict.lisp:1354-1355 ((segments ...) (wi-list* ...)) — map over segments
    let mut wi_list_star: Vec<WordInfo> = Vec::with_capacity(segment_list.segments.len());
    for seg in segment_list.segments.iter_mut() {
        wi_list_star.push(word_info_from_segment(ctx, seg).await?);
    }

    // dict.lisp:1356 (wi1 (car wi-list*)) — bound BEFORE the score filter;
    // every "return wi1 fields" reference below resolves against this.
    let wi1 = wi_list_star
        .first()
        .expect("segment-list has zero segments")
        .clone();
    let matches = segment_list.matches as i32;

    // dict.lisp:1357-1361 (max-score / wi-list = remove-if score < cutoff*max-score)
    // — Lisp `(* 2/3 nil)` and `(< nil _)` both raise TYPE-ERROR; the
    // Rust port panics in the same situation rather than silently
    // substituting 0 (which would change the surviving set).
    let max_int = wi1
        .score
        .expect("word-info-from-segment-list: wi1.score is nil — Lisp `(* 2/3 nil)` would type-error")
        as i64;
    let (num, den) = SEGMENT_SCORE_CUTOFF;
    let wi_list: Vec<WordInfo> = wi_list_star
        .into_iter()
        .filter(|wi| {
            let s = wi.score.expect(
                "word-info-from-segment-list: wi.score is nil during cutoff filter — Lisp `(< nil _)` would type-error",
            ) as i64;
            den * s >= num * max_int
        })
        .collect();

    // dict.lisp:1363-1365 ((if (= (length wi-list) 1) (prog1 wi1 (setf skipped ...))))
    // — `prog1` returns wi1 (the pre-filter binding); we mutate skipped
    // on the wi1 we already cloned above.
    if wi_list.len() == 1 {
        let mut result = wi1;
        result.skipped = matches - 1;
        return Ok(result);
    }

    // dict.lisp:1366-1380 (multi-branch)
    // dict.lisp:1367-1368 — collect kana / seq per child, position-aligned.
    let kana_list: Vec<Option<WordInfoKana>> =
        wi_list.iter().map(|wi| wi.kana.clone()).collect();
    let seq_list: Vec<Option<WordInfoSeq>> =
        wi_list.iter().map(|wi| wi.seq.clone()).collect();

    // dict.lisp:1372 (remove-duplicates kana-list :test 'equal :from-end t)
    let kana_dedup = dedup_keep_first(&kana_list);

    let kept = wi_list.len() as i32;
    Ok(WordInfo {
        // dict.lisp:1370 (:type (word-info-type wi1))
        kind: wi1.kind,
        // dict.lisp:1371 (:text (word-info-text wi1))
        text: wi1.text.clone(),
        kana: Some(WordInfoKana::Multi(kana_dedup)),
        seq: Some(WordInfoSeq::Multi(seq_list)),
        components: wi_list,
        alternative: true,
        // dict.lisp:1376 (:score (word-info-score wi1))
        score: wi1.score,
        start: Some(segment_list.start),
        end: Some(segment_list.end),
        skipped: matches - kept,
        ..Default::default()
    })
}

// dict.lisp:1372 (remove-duplicates kana-list :test 'equal :from-end t)
// — :from-end t keeps the FIRST occurrence in left-to-right order. The
// list is heterogeneous (Single / Multi / None entries), so dedup runs
// on the full Option<WordInfoKana> value via PartialEq.
fn dedup_keep_first(items: &[Option<WordInfoKana>]) -> Vec<Option<WordInfoKana>> {
    let mut out: Vec<Option<WordInfoKana>> = Vec::with_capacity(items.len());
    for item in items {
        if !out.iter().any(|seen| seen == item) {
            out.push(item.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //!
    //! Per-branch coverage:
    //! - single-survivor returns `wi1` with `skipped = matches - 1`;
    //! - multi-survivor builds the synthetic with `alternative=true`,
    //!   `score = wi1.score`, `start/end` from the segment-list, and
    //!   per-child `kana` / `seq` preserved (no flattening);
    //! - the score cutoff anchors on `wi1.score` (the FIRST pre-filter
    //!   wi) — even when wi1 itself survives by tie, callers see the
    //!   wi1 binding.
    //! - `dedup_keep_first` keeps first occurrence on a heterogeneous
    //!   `Vec<Option<WordInfoKana>>` (logic-only, no DB).

    use super::*;
    use crate::dict::find_word::{find_word, FindWordRows};
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment_struct::Segment;
    use crate::dict::word_info_class::WordInfoType;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn one_kana_reading(ctx: &KaniranContext, word: &str) -> KaniWordDispatchEnum {
        let rows = find_word(ctx, word, false).await.unwrap();
        match rows {
            FindWordRows::Kanji(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kanji)
                .expect("at least one kanji-text row"),
            FindWordRows::Kana(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kana)
                .expect("at least one kana-text row"),
        }
    }

    fn seg(word: KaniWordDispatchEnum, score: i32, start: usize, end: usize) -> Segment {
        Segment {
            start,
            end,
            word,
            score: Some(score),
            info: None,
            top: None,
            text: None,
        }
    }

    fn seg_list(segments: Vec<Segment>, start: usize, end: usize, matches: usize) -> SegmentList {
        SegmentList {
            segments,
            start,
            end,
            top: None,
            matches,
        }
    }

    #[tokio::test]
    async fn single_survivor_returns_wi1_with_skipped() {
        // 1 segment, matches=1 → single branch, skipped = matches - 1 = 0.
        let ctx = ctx_from_env().await;
        let word = one_kana_reading(&ctx, "ねこ").await;
        let mut sl = seg_list(vec![seg(word, 16, 0, 2)], 0, 2, 1);
        let wi = word_info_from_segment_list(&ctx, &mut sl).await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kana);
        assert_eq!(wi.text, "ねこ");
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(1467640)));
        assert!(!wi.alternative);
        assert_eq!(wi.skipped, 0);
        assert!(wi.components.is_empty());
        assert_eq!(wi.score, Some(16));
        assert_eq!(wi.start, Some(0));
        assert_eq!(wi.end, Some(2));
    }

    #[tokio::test]
    async fn single_survivor_skipped_eq_matches_minus_one() {
        // After cull-segments, matches > len(wi-list)==1 → skipped = matches - 1.
        let ctx = ctx_from_env().await;
        let word = one_kana_reading(&ctx, "ねこ").await;
        let mut sl = seg_list(vec![seg(word, 16, 0, 2)], 0, 2, 7);
        let wi = word_info_from_segment_list(&ctx, &mut sl).await.unwrap();
        assert_eq!(wi.skipped, 6);
    }

    #[tokio::test]
    async fn multi_survivor_builds_synthetic_from_wi1() {
        // Two surviving segments (scores 5, 5; max=5, cutoff = 2*5/3
        // — both pass with cross-mul 3*5 >= 2*5). Synthetic wi.kind /
        // .text / .score all come from wi1 (the first pre-filter wi).
        let ctx = ctx_from_env().await;
        let neko = one_kana_reading(&ctx, "ねこ").await;
        let inu = one_kana_reading(&ctx, "いぬ").await;
        let mut sl = seg_list(
            vec![seg(neko, 5, 0, 2), seg(inu, 5, 0, 2)],
            0,
            2,
            2,
        );
        let wi = word_info_from_segment_list(&ctx, &mut sl).await.unwrap();
        assert!(wi.alternative);
        assert_eq!(wi.text, "ねこ"); // wi1.text (first segment's wi)
        assert_eq!(wi.score, Some(5));
        assert_eq!(wi.components.len(), 2);
        assert_eq!(wi.start, Some(0));
        assert_eq!(wi.end, Some(2));
        assert_eq!(wi.skipped, 0);
        assert_eq!(
            wi.seq,
            Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(1467640)),
                Some(WordInfoSeq::Single(1258330)),
            ]))
        );
    }

    #[tokio::test]
    async fn multi_branch_filters_below_two_thirds_of_wi1_score() {
        // wi1.score = 9, cutoff = (2*9)/3 = 6. Score-5 and score-3
        // segments fail; only wi1 survives → falls back to single branch.
        let ctx = ctx_from_env().await;
        let a = one_kana_reading(&ctx, "ねこ").await;
        let b = one_kana_reading(&ctx, "いぬ").await;
        let c = one_kana_reading(&ctx, "とり").await;
        let mut sl = seg_list(
            vec![seg(a, 9, 0, 1), seg(b, 5, 0, 1), seg(c, 3, 0, 1)],
            0,
            1,
            3,
        );
        let wi = word_info_from_segment_list(&ctx, &mut sl).await.unwrap();
        assert!(!wi.alternative);
        assert_eq!(wi.text, "ねこ");
        assert_eq!(wi.skipped, 2);
    }

    #[tokio::test]
    async fn multi_branch_anchors_kind_text_score_on_pre_filter_wi1() {
        // Constructed scenario: wi1 has the highest score; second
        // segment also passes the 2/3 cutoff. Confirms wi.text and
        // wi.score follow wi1 even when later survivors have different
        // scores.
        let ctx = ctx_from_env().await;
        let a = one_kana_reading(&ctx, "ねこ").await;
        let b = one_kana_reading(&ctx, "いぬ").await;
        let mut sl = seg_list(
            vec![seg(a, 9, 0, 1), seg(b, 7, 0, 1)],
            0,
            1,
            2,
        );
        let wi = word_info_from_segment_list(&ctx, &mut sl).await.unwrap();
        assert!(wi.alternative);
        assert_eq!(wi.text, "ねこ"); // wi1.text
        assert_eq!(wi.score, Some(9)); // wi1.score
    }

    #[test]
    fn dedup_keep_first_handles_heterogeneous_options() {
        // remove-duplicates :from-end t keeps first occurrence; the
        // collection is heterogeneous (Single, Multi, None).
        let a = Some(WordInfoKana::Single("a".into()));
        let b = Some(WordInfoKana::Single("b".into()));
        let nested = Some(WordInfoKana::Multi(vec![
            Some(WordInfoKana::Single("x".into())),
            Some(WordInfoKana::Single("y".into())),
        ]));
        let none: Option<WordInfoKana> = None;
        let result = dedup_keep_first(&[
            a.clone(),
            b.clone(),
            a.clone(),
            none.clone(),
            nested.clone(),
            none.clone(),
            nested.clone(),
        ]);
        assert_eq!(result, vec![a, b, none, nested]);
    }
}
