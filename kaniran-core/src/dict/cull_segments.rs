//! Port of `ichiran/dict:cull-segments` (`dict.lisp:1027`).
//!
//! Sorts segments by [`compare_common`] over each segment's
//! `info.common` key, then by descending [`Segment::score`], then keeps
//! the leading run whose score is at least `max-score * 1/2`. Empty
//! input returns empty.

use super::_star_identical_word_score_cutoff_star_::IDENTICAL_WORD_SCORE_CUTOFF;
use super::compare_common::compare_common;
use super::segment_struct::Segment;

pub fn cull_segments(mut segments: Vec<Segment>) -> Vec<Segment> {
    if segments.is_empty() {
        return segments;
    }
    // dict.lisp:1029-1030 (stable-sort by compare-common over :common)
    segments.sort_by(|a, b| {
        let ka = a.info.as_ref().and_then(|info| info.common).map(i64::from);
        let kb = b.info.as_ref().and_then(|info| info.common).map(i64::from);
        if compare_common(ka, kb).is_truthy() {
            std::cmp::Ordering::Less
        } else if compare_common(kb, ka).is_truthy() {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
    // dict.lisp:1031 (stable-sort by > on segment-score)
    segments.sort_by(|a, b| b.score.cmp(&a.score));
    // dict.lisp:1032-1033 (max-score / cutoff)
    let max_score = i64::from(segments[0].score.expect(
        "cull-segments: segments[0].score is None — gen-score must populate scores before cull-segments",
    ));
    let (num, den) = IDENTICAL_WORD_SCORE_CUTOFF;
    // dict.lisp:1034-1036 (loop while (>= score cutoff) collect)
    let kept = segments
        .iter()
        .position(|seg| {
            let s = i64::from(
                seg.score
                    .expect("cull-segments: segment.score is None — gen-score must populate"),
            );
            den * s < num * max_score
        })
        .unwrap_or(segments.len());
    segments.truncate(kept);
    segments
}

#[cfg(test)]
mod tests {
    use super::super::kana_text_dao::KanaText;
    use super::super::kani_word::KaniWordDispatchEnum;
    use super::super::segment_struct::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo};
    use super::super::simple_text_class::SimpleText;
    use super::*;

    fn dummy_word(seq: i32) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn info_with_common(common: Option<i32>) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: Vec::new(),
            seq_set: Vec::new(),
            conj: Vec::new(),
            common,
            score_info: KaniScoreInfo {
                prop_score: 0,
                kanji_break: Vec::new(),
                use_length_bonus: 0,
                split_info: KaniSplitInfo::None,
            },
            kpcl: (false, false, false, false),
        }
    }

    fn seg(seq: i32, score: i32, common: Option<Option<i32>>) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(seq),
            score: Some(score),
            info: common.map(info_with_common),
            top: None,
            text: None,
        }
    }

    fn scores(segs: &[Segment]) -> Vec<i32> {
        segs.iter().map(|s| s.score.unwrap()).collect()
    }

    fn seqs(segs: &[Segment]) -> Vec<i32> {
        segs.iter()
            .map(|s| match &s.word {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => unreachable!(),
            })
            .collect()
    }

    // REPL T1: (cull-segments nil) => NIL.
    #[test]
    fn empty_input_returns_empty() {
        let out = cull_segments(Vec::new());
        assert!(out.is_empty());
    }

    // REPL T2: single segment passes through.
    //   IN: [(score 10)] -> OUT: [(score 10)]
    #[test]
    fn single_segment_passes_through() {
        let out = cull_segments(vec![seg(1, 10, None)]);
        assert_eq!(scores(&out), vec![10]);
        assert_eq!(seqs(&out), vec![1]);
    }

    // REPL T3: descending scores with culling.
    //   IN scores [20, 15, 9, 8] -> max=20 cutoff=10 -> OUT [20, 15].
    #[test]
    fn descending_scores_cull_below_half() {
        let out = cull_segments(vec![
            seg(1, 20, None),
            seg(2, 15, None),
            seg(3, 9, None),
            seg(4, 8, None),
        ]);
        assert_eq!(scores(&out), vec![20, 15]);
        assert_eq!(seqs(&out), vec![1, 2]);
    }

    // REPL T4: identical scores — none culled, order preserved.
    //   IN scores [10, 10, 10] -> OUT [10, 10, 10].
    #[test]
    fn identical_scores_none_culled() {
        let out = cull_segments(vec![
            seg(1, 10, None),
            seg(2, 10, None),
            seg(3, 10, None),
        ]);
        assert_eq!(scores(&out), vec![10, 10, 10]);
        assert_eq!(seqs(&out), vec![1, 2, 3]);
    }

    // REPL T5: unsorted input sorted by score desc.
    //   IN scores [5, 20, 15, 12] -> sorted [20, 15, 12, 5] -> max=20
    //   cutoff=10 -> OUT [20, 15, 12].
    #[test]
    fn unsorted_input_sorted_descending() {
        let out = cull_segments(vec![
            seg(1, 5, None),
            seg(2, 20, None),
            seg(3, 15, None),
            seg(4, 12, None),
        ]);
        assert_eq!(scores(&out), vec![20, 15, 12]);
        assert_eq!(seqs(&out), vec![2, 3, 4]);
    }

    // REPL T6: same score, varying :common — compare-common is the
    // primary sort key but score (all equal) is the secondary.
    // Input order [nil, 0, 10, 5] (commons), all score=10.
    //   Expected sorted by compare-common then stable score:
    //   [5, 10, 0, nil] per REPL.
    #[test]
    fn same_score_varying_commons() {
        let out = cull_segments(vec![
            seg(1, 10, Some(None)),
            seg(2, 10, Some(Some(0))),
            seg(3, 10, Some(Some(10))),
            seg(4, 10, Some(Some(5))),
        ]);
        assert_eq!(scores(&out), vec![10, 10, 10, 10]);
        // REPL output order: commons [5, 10, 0, nil] -> seqs [4, 3, 2, 1].
        assert_eq!(seqs(&out), vec![4, 3, 2, 1]);
    }

    // REPL T7: boundary — max=10 cutoff=5; score 5 stays (>= 5), 4
    // dropped.
    //   IN [10, 5, 4] -> OUT [10, 5].
    #[test]
    fn boundary_cutoff_equal_kept() {
        let out = cull_segments(vec![
            seg(1, 10, None),
            seg(2, 5, None),
            seg(3, 4, None),
        ]);
        assert_eq!(scores(&out), vec![10, 5]);
    }

    // REPL T8: odd boundary — max=11 cutoff=11/2=5.5; 6 stays, 5
    // dropped.
    //   IN [11, 6, 5] -> OUT [11, 6].
    #[test]
    fn odd_boundary_cutoff_strict() {
        let out = cull_segments(vec![
            seg(1, 11, None),
            seg(2, 6, None),
            seg(3, 5, None),
        ]);
        assert_eq!(scores(&out), vec![11, 6]);
    }

    // REPL T9: odd boundary with 5 below 5.5.
    //   IN [11, 5] -> OUT [11].
    #[test]
    fn odd_boundary_drops_below_half() {
        let out = cull_segments(vec![seg(1, 11, None), seg(2, 5, None)]);
        assert_eq!(scores(&out), vec![11]);
    }

    // REPL T10: max=10 cutoff=5; score 5 kept.
    //   IN [10, 5] -> OUT [10, 5].
    #[test]
    fn even_boundary_keeps_exactly_half() {
        let out = cull_segments(vec![seg(1, 10, None), seg(2, 5, None)]);
        assert_eq!(scores(&out), vec![10, 5]);
    }

    // REPL T11: zero scores — cutoff 0, all kept (0 >= 0).
    //   IN [0, 0] -> OUT [0, 0].
    #[test]
    fn zero_scores_all_kept() {
        let out = cull_segments(vec![seg(1, 0, None), seg(2, 0, None)]);
        assert_eq!(scores(&out), vec![0, 0]);
    }

    // REPL T12: negative scores — max=-5 cutoff=-2.5; -5 NOT >= -2.5
    // so loop terminates at first segment.
    //   IN [-10, -5] -> sorted [-5, -10] -> OUT [].
    #[test]
    fn negative_scores_all_culled() {
        let out = cull_segments(vec![seg(1, -10, None), seg(2, -5, None)]);
        assert!(out.is_empty());
    }

    // REPL T13: compare-common ordering on commons [nil, 5, 0, 3]
    // with all score=10. Result order (commons): [3, 5, 0, nil] per
    // REPL probe — exercises every compare-common branch:
    //   - 3 < 5 (third clause T)
    //   - 5 < 0 (second clause T)
    //   - 0 < nil (first clause returns 0, truthy)
    //   - nil never sorts before anything (first clause returns nil).
    #[test]
    fn compare_common_ordering_full() {
        let out = cull_segments(vec![
            seg(1, 10, Some(None)),
            seg(2, 10, Some(Some(5))),
            seg(3, 10, Some(Some(0))),
            seg(4, 10, Some(Some(3))),
        ]);
        // REPL order: commons [3, 5, 0, nil] -> seqs [4, 2, 3, 1].
        assert_eq!(seqs(&out), vec![4, 2, 3, 1]);
    }
}
