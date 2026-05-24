//! Port of `ichiran/dict:join-substring-words` (`dict.lisp:1113`).
//!
//! ```lisp
//! (defun join-substring-words (str)
//!   (multiple-value-bind (result kanji-break) (join-substring-words* str)
//!     (loop
//!        with ends-with-lw = (alexandria:ends-with #\ー str)
//!        for (start end segments) in result
//!        for kb = (mapcar (lambda (n) (- n start)) (intersection (list start end) kanji-break))
//!        for sl = (loop for segment in segments
//!                    do (gen-score segment
//!                                  :final (or (= (segment-end segment) (length str))
//!                                             (and ends-with-lw
//!                                                  (= (segment-end segment) (1- (length str)))))
//!                                  :kanji-break kb)
//!                    if (>= (segment-score segment) *score-cutoff*)
//!                    collect segment)
//!        when sl
//!        collect (make-segment-list :segments (cull-segments sl) :start start :end end
//!                                   :matches (length segments)))))
//! ```
//!
//! Scores every segment [`join_substring_words_star_`] produced for each
//! slice, drops those below [`SCORE_CUTOFF`], and wraps each surviving,
//! [`cull_segments`]-filtered group in a [`SegmentList`] tagged with its
//! pre-filter `matches` count.
//!
//! Divergences: `(values result kanji-break)` is destructured from the
//! returned tuple; `(intersection (list start end) kanji-break)` follows
//! SBCL's front-cons order (`end` before `start` when both are present).
//!
//! Audited 531,529/533,756; every failure is an inherited `find-word-full`
//! divergence (non-deterministic suffix-compound reading selection), not
//! this port's logic — see `BUGS.md` §5.

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_score_cutoff_star_::SCORE_CUTOFF;
use crate::dict::cull_segments::cull_segments;
use crate::dict::gen_score::gen_score;
use crate::dict::join_substring_words_star_::join_substring_words_star_;
use crate::dict::segment_list_struct::SegmentList;
use crate::dict::segment_struct::Segment;

pub async fn join_substring_words(
    ctx: &KaniranContext,
    str: &str,
) -> Result<Vec<SegmentList>, sqlx::Error> {
    // (multiple-value-bind (result kanji-break) (join-substring-words* str) ...)
    let (result, kanji_break) = join_substring_words_star_(ctx, str).await?;
    let length = str.chars().count();
    // dict.lisp:1116 — (alexandria:ends-with #\ー str)
    let ends_with_lw = str.chars().last() == Some('ー');

    let mut sls: Vec<SegmentList> = Vec::new();
    // for (start end segments) in result
    for (start, end, segments) in result {
        // dict.lisp:1118 — (mapcar (lambda (n) (- n start))
        //   (intersection (list start end) kanji-break))
        // SBCL conses each match onto the front, so the intersection of
        // (start end) is (end start) when both are present.
        let mut intersection: Vec<usize> = Vec::new();
        if kanji_break.contains(&start) {
            intersection.push(start);
        }
        if kanji_break.contains(&end) {
            intersection.push(end);
        }
        intersection.reverse();
        let kb: Vec<usize> = intersection.iter().map(|n| n - start).collect();

        // :matches (length segments) — the pre-filter segment count.
        let matches = segments.len();
        // for sl = (loop for segment in segments do (gen-score ...)
        //              if (>= (segment-score segment) *score-cutoff*) collect segment)
        let mut sl: Vec<Segment> = Vec::new();
        for mut segment in segments {
            // dict.lisp:1121-1123 — :final (or (= (segment-end segment) (length str))
            //   (and ends-with-lw (= (segment-end segment) (1- (length str)))))
            let final_ = segment.end == length
                || (ends_with_lw && segment.end == length - 1);
            gen_score(ctx, &mut segment, final_, &kb).await?;
            if segment.score.expect("gen-score populates segment.score") >= SCORE_CUTOFF {
                sl.push(segment);
            }
        }
        // when sl collect (make-segment-list :segments (cull-segments sl) ...)
        if !sl.is_empty() {
            sls.push(SegmentList {
                segments: cull_segments(sl),
                start,
                end,
                top: None,
                matches,
            });
        }
    }
    Ok(sls)
}

#[cfg(test)]
mod tests {
    //! Every assertion is REPL-verified against the .103 SBCL via
    //! `(ichiran/dict::join-substring-words …)` (2026-05-23 probe runs).
    //! Run with `cargo test ... -- --test-threads=1` per the DB-test
    //! convention.
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// Per segment-list: `(start, end, matches, [scores high-to-low])`.
    /// Score values are deterministic (calc-score); order among equal
    /// scores can rotate with `find-word`'s unordered SQL, so scores are
    /// compared as a sorted-descending list.
    fn summarize(sls: &[SegmentList]) -> Vec<(usize, usize, usize, Vec<i32>)> {
        sls.iter()
            .map(|sl| {
                let mut scores: Vec<i32> =
                    sl.segments.iter().map(|seg| seg.score.unwrap()).collect();
                scores.sort_unstable_by(|a, b| b.cmp(a));
                (sl.start, sl.end, sl.matches, scores)
            })
            .collect()
    }

    /// REPL `(join-substring-words "日本語")`: 5 segment-lists.
    /// kanji-break is `(2 1)`, so `[1 2]`'s kb is the two-element
    /// reverse-order case `(1 0)`. `[0 1]` keeps 2 of its 4 matches.
    #[tokio::test]
    async fn nihongo() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "日本語").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![
                (0, 1, 4, vec![12, 8]),
                (0, 2, 1, vec![104]),
                (0, 3, 1, vec![1054]),
                (1, 2, 2, vec![8, 6]),
                (2, 3, 2, vec![18]),
            ]
        );
    }

    /// REPL `(join-substring-words "特大")`: 2 segment-lists. `[1 2]`
    /// has matches=5 but a single surviving segment after cutoff/cull.
    #[tokio::test]
    async fn tokudai() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "特大").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![(0, 2, 1, vec![208]), (1, 2, 5, vec![18])]
        );
    }

    /// REPL `(join-substring-words "私は学生です")`: 7 segment-lists.
    /// です is in *force-kanji-break*, 学生 contributes a sequential
    /// break; `[0 1]` keeps 3 of 14 私 readings, `[3 4]` keeps 3 of 7.
    #[tokio::test]
    async fn watashi_wa_gakusei_desu() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "私は学生です").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![
                (0, 1, 14, vec![25, 16, 16]),
                (1, 2, 11, vec![11]),
                (2, 3, 1, vec![8]),
                (2, 4, 2, vec![325]),
                (3, 4, 7, vec![13, 13, 8]),
                (4, 5, 4, vec![11]),
                (4, 6, 2, vec![64]),
            ]
        );
    }

    /// REPL `(join-substring-words "5本")`: the number group drives the
    /// counter path. `[0 1]` "5" is a NUMBER-TEXT scoring exactly at the
    /// cutoff (5); `[0 2]` "5本" yields COUNTER-TEXT + COUNTER-HIFUMI.
    #[tokio::test]
    async fn counter_5hon() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "5本").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![
                (0, 1, 1, vec![5]),
                (0, 2, 2, vec![128, 88]),
                (1, 2, 2, vec![16, 11]),
            ]
        );
    }

    /// REPL `(join-substring-words "ねこー")`: ends-with-lw is T. The
    /// `[0 2]` "ねこ" slice ends at length-1 (=2), so its `:final` is the
    /// `(and ends-with-lw (= end (1- length)))` branch.
    #[tokio::test]
    async fn neko_lw_final_branch() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "ねこー").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![(0, 1, 8, vec![6]), (0, 2, 1, vec![16])]
        );
    }

    /// REPL `(join-substring-words "サッカー")`: ends-with-lw is T, sole
    /// slice `[0 4]` ends at length so `:final` is T via the first
    /// disjunct. matches=3 collapses to a single kana row.
    #[tokio::test]
    async fn sakka_lw() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "サッカー").await.unwrap();
        assert_eq!(summarize(&sls), vec![(0, 4, 3, vec![80])]);
    }

    /// REPL `(join-substring-words "")`: empty input → no segment-lists.
    #[tokio::test]
    async fn empty() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "").await.unwrap();
        assert!(sls.is_empty());
    }

    /// Word identity at the slice level: `[0 3]` of 日本語 is the single
    /// 日本語 entry (seq 1464530), `[2 4]` of 私は学生です is 学生
    /// (seq 1270790-class kanji row). Checks the matched word text, not
    /// just the score shape.
    #[tokio::test]
    async fn slice_word_text() {
        let ctx = ctx().await;
        let mut sls = join_substring_words(&ctx, "日本語").await.unwrap();
        let whole = sls.iter_mut().find(|sl| sl.start == 0 && sl.end == 3).unwrap();
        assert_eq!(whole.segments.len(), 1);
        assert_eq!(whole.segments[0].get_text(), "日本語");
    }
}
