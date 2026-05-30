//! Port of `ichiran/dict:get-seg-initial` (`dict.lisp:1171-1173`).
//!
//! ```lisp
//! (defun get-seg-initial (seg)
//!   (loop for split in (apply-segfilters nil seg)
//!      collect (cadr split)))
//! ```

use std::sync::Arc;

use super::apply_segfilters::apply_segfilters;
use super::kani::KaniLiteSegmentList;

pub fn get_seg_initial(seg: &Arc<KaniLiteSegmentList>) -> Vec<Arc<KaniLiteSegmentList>> {
    apply_segfilters(None, seg)
        .into_iter()
        .map(|(_left, right)| right)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment_list_struct::SegmentList;
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

    fn info_with_seq_set(seq_set: Vec<i32>) -> KaniSegmentInfo {
        KaniSegmentInfo {
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
        }
    }

    fn seg(start: usize, end: usize, seq_set: Vec<i32>) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(info_with_seq_set(seq_set)),
            top: None,
            text: Some(String::new()),
        }
    }

    fn lite_sl(
        start: usize,
        end: usize,
        matches: usize,
        segments: Vec<Segment>,
    ) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches,
        }))
    }

    fn assert_seq_sets(actual: &KaniLiteSegmentList, expected: &[Vec<i32>]) {
        assert_eq!(actual.segments.len(), expected.len());
        for (i, exp) in expected.iter().enumerate() {
            assert_eq!(&actual.segments[i].seq_set, exp, "segments[{}]", i);
        }
    }

    #[test]
    fn a1_empty_segment_list_returns_passthrough() {
        let r = lite_sl(0, 0, 0, vec![]);
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].start, 0);
        assert_eq!(got[0].end, 0);
        assert!(got[0].segments.is_empty());
    }

    #[test]
    fn a2_seq_not_in_any_segfilter_returns_one_unchanged() {
        let r = lite_sl(0, 2, 0, vec![seg(0, 2, vec![999])]);
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        assert_seq_sets(&got[0], &[vec![999]]);
    }

    #[test]
    fn a3_aux_verb_only_seg_yields_zero_splits() {
        let r = lite_sl(0, 2, 0, vec![seg(0, 2, vec![1342560])]);
        let got = get_seg_initial(&r);
        assert!(got.is_empty());
    }

    #[test]
    fn a4_matches_field_carries_through_unchanged() {
        let r = lite_sl(0, 2, 7, vec![seg(0, 2, vec![999])]);
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].matches, 7);
        assert_seq_sets(&got[0], &[vec![999]]);
    }

    #[test]
    fn a5_mixed_aux_and_normal_yields_filtered_subset() {
        // dict-grammar.lisp:1047-1054 — seg-left=nil + non-empty
        // satisfies-right → clause-2 pushes (nil, mslf(r, contradicts-right)).
        let r = lite_sl(
            0,
            2,
            0,
            vec![seg(0, 2, vec![1342560]), seg(0, 2, vec![999])],
        );
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].segments.len(), 1);
        assert!(got[0].segments[0].seq_set.contains(&999));
        assert!(!got[0].segments[0].seq_set.contains(&1342560));
    }
}
