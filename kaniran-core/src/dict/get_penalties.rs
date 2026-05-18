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
//! - The heterogeneous Lisp return list (mixing [`SegmentList`] and
//!   [`Synergy`] values) is modeled as [`Vec<PathElement>`] per
//!   CONVENTIONS §4.3 — the same closed enum the downstream
//!   `find-best-path` path payload uses (see
//!   [`crate::dict::top_array_item_struct::PathElement`]).
//!
//! [`PENALTY_LIST`]: super::_star_penalty_list_star_::PENALTY_LIST
//! [`SegmentList`]: super::segment_list_struct::SegmentList
//! [`Synergy`]: super::synergy_struct::Synergy

use super::_star_penalty_list_star_::PENALTY_LIST;
use super::segment_list_struct::SegmentList;
use super::top_array_item_struct::PathElement;

pub fn get_penalties(seg_left: &SegmentList, seg_right: &SegmentList) -> Vec<PathElement> {
    for penalty_fn in PENALTY_LIST {
        if let Some(penalty) = penalty_fn(seg_left, seg_right) {
            // dict-grammar.lisp:1015 (`(return (list seg-right penalty seg-left))`)
            return vec![
                PathElement::SegmentList(seg_right.clone()),
                PathElement::Synergy(penalty),
                PathElement::SegmentList(seg_left.clone()),
            ];
        }
    }
    // dict-grammar.lisp:1016 (`(finally (return (list seg-right seg-left)))`)
    vec![
        PathElement::SegmentList(seg_right.clone()),
        PathElement::SegmentList(seg_left.clone()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::_star_semi_final_prt_star_::semi_final_prt;
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

    fn sl(start: usize, end: usize, segments: Vec<Segment>) -> SegmentList {
        SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }
    }

    // REPL probes (/tmp/probe_penalties.lisp on .103, 2026-05-18).

    #[test]
    fn a_no_penalty_returns_two_element_list() {
        // probe A: long, kpcl-flagged segs → both penalties fail →
        // 2-element list `(seg-right seg-left)`.
        let l = sl(0, 3, vec![seg(0, 3, (true, false, false, false), vec![999], "abc")]);
        let r = sl(3, 6, vec![seg(3, 6, (true, false, false, false), vec![888], "def")]);
        let res = get_penalties(&l, &r);
        assert_eq!(res.len(), 2);
        match &res[0] {
            PathElement::SegmentList(sl) => {
                assert_eq!(sl.start, 3);
                assert_eq!(sl.end, 6);
                assert_eq!(sl.segments.len(), 1);
                assert_eq!(sl.segments[0].text.as_deref(), Some("def"));
            }
            other => panic!("expected SegmentList at [0], got {:?}", other),
        }
        match &res[1] {
            PathElement::SegmentList(sl) => {
                assert_eq!(sl.start, 0);
                assert_eq!(sl.end, 3);
                assert_eq!(sl.segments.len(), 1);
                assert_eq!(sl.segments[0].text.as_deref(), Some("abc"));
            }
            other => panic!("expected SegmentList at [1], got {:?}", other),
        }
    }

    #[test]
    fn b_penalty_short_triggers_returns_three_element_list() {
        // probe B: both 1-char, no kpcl flag → penalty-short fires →
        // 3-element `(seg-right penalty seg-left)`, synergy descr
        // "short" score=-9 start=1 end=3.
        let l = sl(0, 1, vec![seg(0, 1, (false, false, false, false), vec![999], "あ")]);
        let r = sl(3, 4, vec![seg(3, 4, (false, false, false, false), vec![888], "い")]);
        let res = get_penalties(&l, &r);
        assert_eq!(res.len(), 3);
        match &res[0] {
            PathElement::SegmentList(sl) => {
                assert_eq!(sl.start, 3);
                assert_eq!(sl.end, 4);
                assert_eq!(sl.segments[0].text.as_deref(), Some("い"));
            }
            other => panic!("expected SegmentList at [0], got {:?}", other),
        }
        match &res[1] {
            PathElement::Synergy(s) => {
                assert_eq!(s.description.as_deref(), Some("short"));
                assert_eq!(s.connector.as_deref(), Some(" "));
                assert_eq!(s.score, -9);
                assert_eq!(s.start, 1);
                assert_eq!(s.end, 3);
            }
            other => panic!("expected Synergy at [1], got {:?}", other),
        }
        match &res[2] {
            PathElement::SegmentList(sl) => {
                assert_eq!(sl.start, 0);
                assert_eq!(sl.end, 1);
                assert_eq!(sl.segments[0].text.as_deref(), Some("あ"));
            }
            other => panic!("expected SegmentList at [2], got {:?}", other),
        }
    }

    #[test]
    fn c_penalty_semi_final_triggers_returns_three_element_list() {
        // probe C: adjacent (l.end=1=r.start), l seg with seq in
        // *semi-final-prt*. semi-final fires before short (it's first
        // in *penalty-list*). Synergy descr "semi-final not final"
        // score=-15 start=1 end=1.
        let semi_seq = semi_final_prt()[0];
        let l = sl(
            0,
            1,
            vec![seg(0, 1, (false, false, false, false), vec![semi_seq], "x")],
        );
        let r = sl(1, 2, vec![seg(1, 2, (false, false, false, false), vec![999], "y")]);
        let res = get_penalties(&l, &r);
        assert_eq!(res.len(), 3);
        match &res[0] {
            PathElement::SegmentList(sl) => {
                assert_eq!(sl.start, 1);
                assert_eq!(sl.end, 2);
                assert_eq!(sl.segments[0].info.as_ref().unwrap().seq_set, vec![999]);
            }
            other => panic!("expected SegmentList at [0], got {:?}", other),
        }
        match &res[1] {
            PathElement::Synergy(s) => {
                assert_eq!(s.description.as_deref(), Some("semi-final not final"));
                assert_eq!(s.connector.as_deref(), Some(" "));
                assert_eq!(s.score, -15);
                assert_eq!(s.start, 1);
                assert_eq!(s.end, 1);
            }
            other => panic!("expected Synergy at [1], got {:?}", other),
        }
        match &res[2] {
            PathElement::SegmentList(sl) => {
                assert_eq!(sl.start, 0);
                assert_eq!(sl.end, 1);
                assert_eq!(
                    sl.segments[0].info.as_ref().unwrap().seq_set,
                    vec![semi_seq]
                );
            }
            other => panic!("expected SegmentList at [2], got {:?}", other),
        }
    }

    #[test]
    fn d_both_could_match_semi_final_wins_first_in_list() {
        // probe D: same inputs as C — penalty-short would also fire
        // (both spans len=1, no kpcl), but penalty-semi-final is
        // first in *penalty-list* and returns first, so score is
        // -15, not -9.
        let semi_seq = semi_final_prt()[0];
        let l = sl(
            0,
            1,
            vec![seg(0, 1, (false, false, false, false), vec![semi_seq], "x")],
        );
        let r = sl(1, 2, vec![seg(1, 2, (false, false, false, false), vec![999], "y")]);
        let res = get_penalties(&l, &r);
        assert_eq!(res.len(), 3);
        match &res[1] {
            PathElement::Synergy(s) => {
                assert_eq!(s.description.as_deref(), Some("semi-final not final"));
                assert_eq!(s.score, -15);
            }
            other => panic!("expected Synergy at [1], got {:?}", other),
        }
    }

    #[test]
    fn e_penalty_branch_arg_order_seg_right_then_seg_left() {
        // probe E: penalty branch — list is (seg-right penalty seg-left).
        let l = sl(
            10,
            11,
            vec![seg(10, 11, (false, false, false, false), vec![444], "α")],
        );
        let r = sl(
            13,
            14,
            vec![seg(13, 14, (false, false, false, false), vec![555], "β")],
        );
        let res = get_penalties(&l, &r);
        assert_eq!(res.len(), 3);
        match &res[0] {
            PathElement::SegmentList(sl) => {
                assert_eq!(sl.start, 13);
                assert_eq!(sl.end, 14);
                assert_eq!(sl.segments[0].text.as_deref(), Some("β"));
            }
            other => panic!("expected SegmentList at [0], got {:?}", other),
        }
        match &res[1] {
            PathElement::Synergy(s) => {
                assert_eq!(s.description.as_deref(), Some("short"));
                assert_eq!(s.score, -9);
                assert_eq!(s.start, 11);
                assert_eq!(s.end, 13);
            }
            other => panic!("expected Synergy at [1], got {:?}", other),
        }
        match &res[2] {
            PathElement::SegmentList(sl) => {
                assert_eq!(sl.start, 10);
                assert_eq!(sl.end, 11);
                assert_eq!(sl.segments[0].text.as_deref(), Some("α"));
            }
            other => panic!("expected SegmentList at [2], got {:?}", other),
        }
    }

    #[test]
    fn f_no_penalty_branch_arg_order_seg_right_then_seg_left() {
        // probe F: no-penalty branch — list is (seg-right seg-left).
        let l = sl(
            10,
            13,
            vec![seg(10, 13, (true, false, false, false), vec![444], "α-long")],
        );
        let r = sl(
            13,
            16,
            vec![seg(13, 16, (true, false, false, false), vec![555], "β-long")],
        );
        let res = get_penalties(&l, &r);
        assert_eq!(res.len(), 2);
        match &res[0] {
            PathElement::SegmentList(sl) => {
                assert_eq!(sl.start, 13);
                assert_eq!(sl.end, 16);
                assert_eq!(sl.segments[0].text.as_deref(), Some("β-long"));
            }
            other => panic!("expected SegmentList at [0], got {:?}", other),
        }
        match &res[1] {
            PathElement::SegmentList(sl) => {
                assert_eq!(sl.start, 10);
                assert_eq!(sl.end, 13);
                assert_eq!(sl.segments[0].text.as_deref(), Some("α-long"));
            }
            other => panic!("expected SegmentList at [1], got {:?}", other),
        }
    }

    #[test]
    fn g_empty_l_segments_no_penalty_branch() {
        // probe G: l has no segments → both penalty fns' tests bail
        // out → 2-element list, seg-right first.
        let r = sl(1, 2, vec![seg(1, 2, (false, false, false, false), vec![999], "い")]);
        let l = sl(0, 1, vec![]);
        let res = get_penalties(&l, &r);
        assert_eq!(res.len(), 2);
        match &res[0] {
            PathElement::SegmentList(sl) => {
                assert_eq!(sl.start, 1);
                assert_eq!(sl.end, 2);
                assert_eq!(sl.segments.len(), 1);
            }
            other => panic!("expected SegmentList at [0], got {:?}", other),
        }
        match &res[1] {
            PathElement::SegmentList(sl) => {
                assert_eq!(sl.start, 0);
                assert_eq!(sl.end, 1);
                assert_eq!(sl.segments.len(), 0);
            }
            other => panic!("expected SegmentList at [1], got {:?}", other),
        }
    }
}
