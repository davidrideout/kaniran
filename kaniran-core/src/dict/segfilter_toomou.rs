//! Port of `ichiran/dict:segfilter-toomou` (`dict-grammar.lisp:1132`).
//!
//! Splits と before 思う/言う: keeps a 思う/言う right segment only when
//! the preceding left segment is not 何だと (2837117).

use std::sync::Arc;

use super::def_segfilter_must_follow_macro::def_segfilter_must_follow_body;
use super::filter_in_seq_set::filter_in_seq_set;
use super::kani_lite_segment_list::KaniLiteSegmentList;

const NANDATO_SEQ: i32 = 2837117;
const OMOU_IU_SEQS: &[i32] = &[1589350, 1587040];

pub fn segfilter_toomou(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> {
    let inner = filter_in_seq_set(vec![NANDATO_SEQ]);
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        filter_in_seq_set(OMOU_IU_SEQS.to_vec()),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kana_text_dao::KanaText;
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

    fn seg(start: usize, end: usize, info: KaniSegmentInfo) -> Segment {
        Segment { start, end, word: dummy_word(), score: None, info: Some(info), top: None, text: None }
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

    // REPL probes from `/tmp/probe_415_423.lisp` (this session).

    #[test]
    fn tm_a_l_nil_r_omou_pass_through() {
        // TM-A l=NIL r=omou cnt=1 — allow-first
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![1589350]))]);
        let result = segfilter_toomou(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn tm_b_l_nil_r_no_match() {
        // TM-B l=NIL r=no-match cnt=1 — clause-1
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let result = segfilter_toomou(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn tm_c_l_not_nandato_r_omou() {
        // TM-C l-not-nandato r-omou cnt=1
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1589350]))]);
        let result = segfilter_toomou(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn tm_d_l_nandato_r_omou_empty() {
        // TM-D l-nandato r-omou cnt=0
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2837117]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1589350]))]);
        let result = segfilter_toomou(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn tm_e_l_mixed_r_omou() {
        // TM-E l-mixed r-omou cnt=1 — sat-l push (con-r empty)
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, info_with_seq_set(vec![2837117])),
                seg(0, 1, info_with_seq_set(vec![999])),
            ],
        );
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1589350]))]);
        let result = segfilter_toomou(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![999]);
        assert_eq!(result[0].1.segments.len(), 1);
    }
}
