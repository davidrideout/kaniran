//! Port of `ichiran/dict:apply-segfilters` (`dict-grammar.lisp:1170`).
//!
//! Threads `(seg-left, seg-right)` through each filter in
//! [`SEGFILTER_LIST`] in order. Each filter returns a list of
//! `(seg-left, seg-right)` candidates; the union of those candidates
//! becomes the input to the next filter.
//!
//! [`SEGFILTER_LIST`]: super::_star_segfilter_list_star_::SEGFILTER_LIST

use std::sync::Arc;

use super::_star_segfilter_list_star_::SEGFILTER_LIST;
use super::kani_lite_segment_list::KaniLiteSegmentList;

pub fn apply_segfilters(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> {
    // dict-grammar.lisp:1171 (`with splits = (list (list seg-left seg-right))`)
    let mut splits: Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> =
        vec![(seg_left.cloned(), Arc::clone(seg_right))];
    for segfilter in SEGFILTER_LIST {
        // dict-grammar.lisp:1173-1175 (inner loop nconc-ing each
        // filter's output across the current splits)
        let mut next: Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> =
            Vec::new();
        for (left, right) in &splits {
            next.extend(segfilter(left.as_ref(), right));
        }
        splits = next;
    }
    splits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::conj_prop_dao::ConjProp;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_lite_segment::KaniLiteSegment;
    use crate::dict::kani_word::KaniWordDispatchEnum;
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

    fn cdata(conj_type: i32) -> ConjData {
        ConjData {
            seq: None,
            from: None,
            via: None,
            prop: Some(ConjProp {
                id: 0,
                conj_id: 0,
                conj_type,
                pos: String::new(),
                neg: None,
                fml: None,
            }),
            src_map: vec![],
        }
    }

    fn info_with_seq_set(seq_set: Vec<i32>) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: vec![],
            seq_set,
            conj: vec![],
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

    fn info_with_conj(conj: Vec<ConjData>) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: vec![],
            seq_set: vec![],
            conj,
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

    fn seg(start: usize, end: usize, info: KaniSegmentInfo) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(info),
            top: None,
            text: None,
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes (`/tmp/probe_apply_segfilters.lisp` on .103).

    fn assert_seq_set_seg(seg: &Arc<KaniLiteSegment>, start: usize, end: usize, seq_set: &[i32]) {
        assert_eq!(seg.source.start, start);
        assert_eq!(seg.source.end, end);
        assert_eq!(seg.seq_set, seq_set);
        assert!(seg.conj_types.is_empty());
        assert_eq!(seg.pos, 0);
        assert_eq!(seg.kpcl, 0);
    }

    fn assert_conj_seg(
        seg: &Arc<KaniLiteSegment>,
        start: usize,
        end: usize,
        conj_type: i32,
    ) {
        assert_eq!(seg.source.start, start);
        assert_eq!(seg.source.end, end);
        assert!(seg.seq_set.is_empty());
        assert_eq!(seg.conj_types, vec![conj_type]);
        assert_eq!(seg.pos, 0);
        assert_eq!(seg.kpcl, 0);
    }

    fn assert_sl(
        sl: &KaniLiteSegmentList,
        start: usize,
        end: usize,
        matches: usize,
        n_segs: usize,
    ) {
        assert_eq!(sl.start, start);
        assert_eq!(sl.end, end);
        assert_eq!(sl.matches, matches);
        assert!(sl.top.is_none());
        assert_eq!(sl.segments.len(), n_segs);
    }

    #[test]
    fn a_nil_left_unmatched_right_identity() {
        let r = lite_sl(0, 2, vec![seg(0, 2, info_with_seq_set(vec![999]))]);
        let result = apply_segfilters(None, &r);
        assert_eq!(result.len(), 1);
        let (lp, rp) = &result[0];
        assert!(lp.is_none());
        assert_sl(rp, 0, 2, 0, 1);
        assert_seq_set_seg(&rp.segments[0], 0, 2, &[999]);
    }

    #[test]
    fn b_nil_left_aux_verb_only_right_filtered_to_empty() {
        let r = lite_sl(0, 2, vec![seg(0, 2, info_with_seq_set(vec![1342560]))]);
        let result = apply_segfilters(None, &r);
        assert!(result.is_empty());
    }

    #[test]
    fn c_adjacent_l_conj13_r_aux_verb_full_pair() {
        let l = lite_sl(0, 2, vec![seg(0, 2, info_with_conj(vec![cdata(13)]))]);
        let r = lite_sl(2, 4, vec![seg(2, 4, info_with_seq_set(vec![1342560]))]);
        let result = apply_segfilters(Some(&l), &r);
        assert_eq!(result.len(), 1);
        let (lp, rp) = &result[0];
        let lp_ref = lp.as_ref().unwrap();
        assert_sl(lp_ref, 0, 2, 0, 1);
        assert_sl(rp, 2, 4, 0, 1);
        assert_conj_seg(&lp_ref.segments[0], 0, 2, 13);
        assert_seq_set_seg(&rp.segments[0], 2, 4, &[1342560]);
    }

    #[test]
    fn d_adjacent_l_mixed_conj_r_mixed_aux_two_splits() {
        let l = lite_sl(
            0,
            2,
            vec![
                seg(0, 2, info_with_conj(vec![cdata(13)])),
                seg(0, 2, info_with_conj(vec![cdata(3)])),
            ],
        );
        let r = lite_sl(
            2,
            4,
            vec![
                seg(2, 4, info_with_seq_set(vec![1342560])),
                seg(2, 4, info_with_seq_set(vec![999])),
            ],
        );
        let result = apply_segfilters(Some(&l), &r);
        assert_eq!(result.len(), 2);

        let (lp0, rp0) = &result[0];
        let lp0_ref = lp0.as_ref().unwrap();
        assert_sl(lp0_ref, 0, 2, 0, 1);
        assert_sl(rp0, 2, 4, 0, 1);
        assert_conj_seg(&lp0_ref.segments[0], 0, 2, 13);
        assert_seq_set_seg(&rp0.segments[0], 2, 4, &[1342560]);

        let (lp1, rp1) = &result[1];
        let lp1_ref = lp1.as_ref().unwrap();
        assert_sl(lp1_ref, 0, 2, 0, 2);
        assert_sl(rp1, 2, 4, 0, 1);
        assert_conj_seg(&lp1_ref.segments[0], 0, 2, 13);
        assert_conj_seg(&lp1_ref.segments[1], 0, 2, 3);
        assert_seq_set_seg(&rp1.segments[0], 2, 4, &[999]);
    }

    #[test]
    fn e_nil_left_n_only_right_filtered() {
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2139720]))]);
        let result = apply_segfilters(None, &r);
        assert_eq!(result.len(), 1);
        let (lp, rp) = &result[0];
        assert!(lp.is_none());
        assert_sl(rp, 0, 1, 0, 1);
        assert_seq_set_seg(&rp.segments[0], 0, 1, &[2139720]);
    }
}
