//! Port of `ichiran/dict:synergy-no-toori` (`dict-grammar.lisp:944`).
//!
//! ```lisp
//! (def-generic-synergy synergy-no-toori (l r)
//!   (filter-in-seq-set 1469800)
//!   (filter-in-seq-set 1432920)
//!   :description "no toori"
//!   :score 50
//!   :connector " ")
//! ```
//!
//! Divergences from Lisp:
//! - `pushnew ',name *synergy-list*` from the `defsynergy` expansion
//!   moves to the `*synergy-list*` port (separate wave).

use super::filter_in_seq_set::filter_in_seq_set;
use super::make_segment_list_from::make_segment_list_from;
use super::segment_list_struct::SegmentList;
use super::synergy_struct::Synergy;

pub fn synergy_no_toori(l: &SegmentList, r: &SegmentList) -> Vec<(SegmentList, Synergy, SegmentList)> {
    let start = l.end;
    let end = r.start;
    // dict-grammar.lisp:731-746 (def-generic-synergy expansion)
    if start != end {
        return vec![];
    }
    let test_left = filter_in_seq_set(vec![1469800]);
    let test_right = filter_in_seq_set(vec![1432920]);
    let left: Vec<_> = l.segments.iter().filter(|s| test_left(s)).cloned().collect();
    let right: Vec<_> = r.segments.iter().filter(|s| test_right(s)).cloned().collect();
    if left.is_empty() || right.is_empty() {
        return vec![];
    }
    let syn = Synergy {
        description: Some("no toori".to_string()),
        connector: Some(" ".to_string()),
        score: 50,
        start,
        end,
    };
    vec![(
        make_segment_list_from(r, right),
        syn,
        make_segment_list_from(l, left),
    )]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::segment_struct::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::simple_text_class::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
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

    fn seg_with_seqs(seq_set: Vec<i32>) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: vec![],
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl: (false, false, false, false),
            }),
            top: None,
            text: None,
        }
    }

    fn sl(start: usize, end: usize, segments: Vec<Segment>) -> SegmentList {
        SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }
    }

    // REPL probes (/tmp/probe_synergies.lisp on .103, 2026-05-18).

    #[test]
    fn positive_no_toori() {
        // no-toori/positive: RIGHT-SL start=1 end=3 segs=1,
        // SYNERGY desc="no toori" conn=" " score=50 start=1 end=1,
        // LEFT-SL start=0 end=1 segs=1.
        let l = sl(0, 1, vec![seg_with_seqs(vec![1469800])]);
        let r = sl(1, 3, vec![seg_with_seqs(vec![1432920])]);
        let got = synergy_no_toori(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("no toori"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 50);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn left_misses_empty() {
        // no-toori/left-misses: NIL.
        let l = sl(0, 1, vec![seg_with_seqs(vec![12345])]);
        let r = sl(1, 3, vec![seg_with_seqs(vec![1432920])]);
        assert!(synergy_no_toori(&l, &r).is_empty());
    }

    #[test]
    fn multi_segs_partial_filter() {
        // no-toori/multi-segs-partial: l has 2 segs (one matches, one
        // does not), r has 2 segs (both match). Expected RIGHT-SL
        // segs=2, LEFT-SL segs=1.
        let l = sl(
            0,
            1,
            vec![seg_with_seqs(vec![1469800]), seg_with_seqs(vec![99])],
        );
        let r = sl(
            1,
            3,
            vec![
                seg_with_seqs(vec![1432920]),
                seg_with_seqs(vec![1432920, 88]),
            ],
        );
        let got = synergy_no_toori(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, _syn, left_sl) = &got[0];
        assert_eq!(right_sl.segments.len(), 2);
        assert_eq!(left_sl.segments.len(), 1);
    }
}
