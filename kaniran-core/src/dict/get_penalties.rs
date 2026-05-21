//! Port of `ichiran/dict:get-penalties` (`dict-grammar.lisp:1011-1016`).
//!
//! Walks [`PENALTY_LIST`] in order, returning the first penalty that
//! fires between `seg_left` and `seg_right`. Result is either the
//! three-element `(seg_right, penalty, seg_left)` shape when a
//! penalty matched, or the two-element `(seg_right, seg_left)` shape
//! when no penalty fn returned a [`Synergy`].
//!
//! ```lisp
//! (defun get-penalties (seg-left seg-right)
//!   (loop for fn in *penalty-list*
//!      for penalty = (funcall fn seg-left seg-right)
//!      when penalty
//!        do (return (list seg-right penalty seg-left))
//!      finally (return (list seg-right seg-left))))
//! ```
//!
//! Divergences from Lisp:
//! - The heterogeneous Lisp return list is modeled as
//!   [`Vec<KaniLitePathElement>`] per CONVENTIONS §4.3.
//!
//! [`PENALTY_LIST`]: super::_star_penalty_list_star_::PENALTY_LIST
//! [`Synergy`]: super::synergy_struct::Synergy

use std::sync::Arc;

use super::_star_penalty_list_star_::PENALTY_LIST;
use super::kani_lite_segment_list::KaniLiteSegmentList;
use super::kani_lite_top_array_item::KaniLitePathElement;

pub fn get_penalties(
    seg_left: &Arc<KaniLiteSegmentList>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Vec<KaniLitePathElement> {
    for penalty_fn in PENALTY_LIST {
        if let Some(penalty) = penalty_fn(seg_left, seg_right) {
            // dict-grammar.lisp:1015 (`(return (list seg-right penalty seg-left))`)
            return vec![
                KaniLitePathElement::SegmentList(Arc::clone(seg_right)),
                KaniLitePathElement::Synergy(penalty),
                KaniLitePathElement::SegmentList(Arc::clone(seg_left)),
            ];
        }
    }
    // dict-grammar.lisp:1016 (`(finally (return (list seg-right seg-left)))`)
    vec![
        KaniLitePathElement::SegmentList(Arc::clone(seg_right)),
        KaniLitePathElement::SegmentList(Arc::clone(seg_left)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::_star_semi_final_prt_star_::semi_final_prt;
    use crate::dict::conj_data_struct::ConjData;
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

    fn info_with(kpcl: (bool, bool, bool, bool), seq_set: Vec<i32>) -> KaniSegmentInfo {
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
            kpcl,
        }
    }

    fn seg(
        start: usize,
        end: usize,
        kpcl: (bool, bool, bool, bool),
        seq_set: Vec<i32>,
        text: &str,
    ) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(info_with(kpcl, seq_set)),
            top: None,
            text: Some(text.to_string()),
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

    fn unwrap_sl(elem: &KaniLitePathElement) -> &Arc<KaniLiteSegmentList> {
        match elem {
            KaniLitePathElement::SegmentList(sl) => sl,
            other => panic!("expected SegmentList, got {:?}", other),
        }
    }

    fn unwrap_synergy(elem: &KaniLitePathElement) -> &crate::dict::synergy_struct::Synergy {
        match elem {
            KaniLitePathElement::Synergy(s) => s,
            other => panic!("expected Synergy, got {:?}", other),
        }
    }

    // REPL probes (/tmp/probe_penalties.lisp on .103, 2026-05-18).

    #[test]
    fn a_no_penalty_returns_two_element_list() {
        let l = lite_sl(0, 3, vec![seg(0, 3, (true, false, false, false), vec![999], "abc")]);
        let r = lite_sl(3, 6, vec![seg(3, 6, (true, false, false, false), vec![888], "def")]);
        let res = get_penalties(&l, &r);
        assert_eq!(res.len(), 2);
        let s0 = unwrap_sl(&res[0]);
        assert_eq!(s0.start, 3);
        assert_eq!(s0.end, 6);
        assert_eq!(s0.segments.len(), 1);
        assert_eq!(s0.segments[0].text.as_ref(), "def");
        let s1 = unwrap_sl(&res[1]);
        assert_eq!(s1.start, 0);
        assert_eq!(s1.end, 3);
        assert_eq!(s1.segments[0].text.as_ref(), "abc");
    }

    #[test]
    fn b_penalty_short_triggers_returns_three_element_list() {
        let l = lite_sl(0, 1, vec![seg(0, 1, (false, false, false, false), vec![999], "あ")]);
        let r = lite_sl(3, 4, vec![seg(3, 4, (false, false, false, false), vec![888], "い")]);
        let res = get_penalties(&l, &r);
        assert_eq!(res.len(), 3);
        let s0 = unwrap_sl(&res[0]);
        assert_eq!(s0.start, 3);
        assert_eq!(s0.end, 4);
        assert_eq!(s0.segments[0].text.as_ref(), "い");
        let syn = unwrap_synergy(&res[1]);
        assert_eq!(syn.description.as_deref(), Some("short"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, -9);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 3);
        let s2 = unwrap_sl(&res[2]);
        assert_eq!(s2.start, 0);
        assert_eq!(s2.end, 1);
        assert_eq!(s2.segments[0].text.as_ref(), "あ");
    }

    #[test]
    fn c_penalty_semi_final_triggers_returns_three_element_list() {
        let semi_seq = semi_final_prt()[0];
        let l = lite_sl(0, 1, vec![seg(0, 1, (false, false, false, false), vec![semi_seq], "x")]);
        let r = lite_sl(1, 2, vec![seg(1, 2, (false, false, false, false), vec![999], "y")]);
        let res = get_penalties(&l, &r);
        assert_eq!(res.len(), 3);
        let s0 = unwrap_sl(&res[0]);
        assert_eq!(s0.start, 1);
        assert_eq!(s0.end, 2);
        assert_eq!(s0.segments[0].seq_set, vec![999]);
        let syn = unwrap_synergy(&res[1]);
        assert_eq!(syn.description.as_deref(), Some("semi-final not final"));
        assert_eq!(syn.score, -15);
        let s2 = unwrap_sl(&res[2]);
        assert_eq!(s2.start, 0);
        assert_eq!(s2.end, 1);
        assert_eq!(s2.segments[0].seq_set, vec![semi_seq]);
    }

    #[test]
    fn d_both_could_match_semi_final_wins_first_in_list() {
        let semi_seq = semi_final_prt()[0];
        let l = lite_sl(0, 1, vec![seg(0, 1, (false, false, false, false), vec![semi_seq], "x")]);
        let r = lite_sl(1, 2, vec![seg(1, 2, (false, false, false, false), vec![999], "y")]);
        let res = get_penalties(&l, &r);
        assert_eq!(res.len(), 3);
        let syn = unwrap_synergy(&res[1]);
        assert_eq!(syn.description.as_deref(), Some("semi-final not final"));
        assert_eq!(syn.score, -15);
    }

    #[test]
    fn e_penalty_branch_arg_order_seg_right_then_seg_left() {
        let l = lite_sl(10, 11, vec![seg(10, 11, (false, false, false, false), vec![444], "α")]);
        let r = lite_sl(13, 14, vec![seg(13, 14, (false, false, false, false), vec![555], "β")]);
        let res = get_penalties(&l, &r);
        assert_eq!(res.len(), 3);
        let s0 = unwrap_sl(&res[0]);
        assert_eq!(s0.start, 13);
        assert_eq!(s0.end, 14);
        assert_eq!(s0.segments[0].text.as_ref(), "β");
        let syn = unwrap_synergy(&res[1]);
        assert_eq!(syn.description.as_deref(), Some("short"));
        assert_eq!(syn.score, -9);
        assert_eq!(syn.start, 11);
        assert_eq!(syn.end, 13);
        let s2 = unwrap_sl(&res[2]);
        assert_eq!(s2.start, 10);
        assert_eq!(s2.end, 11);
        assert_eq!(s2.segments[0].text.as_ref(), "α");
    }

    #[test]
    fn f_no_penalty_branch_arg_order_seg_right_then_seg_left() {
        let l = lite_sl(10, 13, vec![seg(10, 13, (true, false, false, false), vec![444], "α-long")]);
        let r = lite_sl(13, 16, vec![seg(13, 16, (true, false, false, false), vec![555], "β-long")]);
        let res = get_penalties(&l, &r);
        assert_eq!(res.len(), 2);
        let s0 = unwrap_sl(&res[0]);
        assert_eq!(s0.start, 13);
        assert_eq!(s0.end, 16);
        assert_eq!(s0.segments[0].text.as_ref(), "β-long");
        let s1 = unwrap_sl(&res[1]);
        assert_eq!(s1.start, 10);
        assert_eq!(s1.end, 13);
        assert_eq!(s1.segments[0].text.as_ref(), "α-long");
    }

    #[test]
    fn g_empty_l_segments_no_penalty_branch() {
        let r = lite_sl(1, 2, vec![seg(1, 2, (false, false, false, false), vec![999], "い")]);
        let l = lite_sl(0, 1, vec![]);
        let res = get_penalties(&l, &r);
        assert_eq!(res.len(), 2);
        let s0 = unwrap_sl(&res[0]);
        assert_eq!(s0.start, 1);
        assert_eq!(s0.end, 2);
        assert_eq!(s0.segments.len(), 1);
        let s1 = unwrap_sl(&res[1]);
        assert_eq!(s1.start, 0);
        assert_eq!(s1.end, 1);
        assert_eq!(s1.segments.len(), 0);
    }
}
