//! Port of `ichiran/dict:segfilter-tsu-iru` (`dict-grammar.lisp:1081`).
//!
//! ```lisp
//! (def-segfilter-must-follow segfilter-tsu-iru (l r) ;; TODO: remove this, or make more generic
//!   (complement (filter-in-seq-set 2221640))
//!   (filter-in-seq-set 1577980)
//!   :allow-first t)
//! ```

use std::sync::Arc;

use super::def_segfilter_must_follow_macro::def_segfilter_must_follow_body;
use super::filter_in_seq_set::filter_in_seq_set;
use super::kani::KaniLiteSegmentList;

const TSU_SEQ: i32 = 2221640;
const IRU_SEQS: &[i32] = &[1577980];

pub fn segfilter_tsu_iru(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> {
    let inner = filter_in_seq_set(vec![TSU_SEQ]);
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        filter_in_seq_set(IRU_SEQS.to_vec()),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn ti_a_l_nil_r_iru_pass_through() {
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![1577980]))]);
        let result = segfilter_tsu_iru(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn ti_b_l_nil_r_no_match() {
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let result = segfilter_tsu_iru(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn ti_c_l_not_tsu_r_iru() {
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1577980]))]);
        let result = segfilter_tsu_iru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn ti_d_l_tsu_r_iru_empty() {
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2221640]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1577980]))]);
        let result = segfilter_tsu_iru(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn ti_e_l_mixed_r_iru() {
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, info_with_seq_set(vec![2221640])),
                seg(0, 1, info_with_seq_set(vec![999])),
            ],
        );
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1577980]))]);
        let result = segfilter_tsu_iru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![999]);
        assert_eq!(result[0].1.segments.len(), 1);
    }
}
