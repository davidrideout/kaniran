//! Port of the dict-grammar.lisp penalty layer.

pub use _star_penalty_list_star__inner::*;
pub use def_generic_penalty_macro_inner::*;
pub use penalty_short_inner::*;
pub use penalty_semi_final_inner::*;
pub use get_penalties_inner::*;

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod _star_penalty_list_star__inner {
use crate::dict::kani::KaniLiteSegmentList;
use super::penalty_semi_final;
use super::penalty_short;
use crate::dict::grammar::synergy::Synergy;

pub type Penalty = fn(&KaniLiteSegmentList, &KaniLiteSegmentList) -> Option<Synergy>;

pub static PENALTY_LIST: &[Penalty] = &[penalty_semi_final, penalty_short];
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod def_generic_penalty_macro_inner {
use crate::dict::kani::KaniLiteSegmentList;
use crate::dict::grammar::synergy::Synergy;

pub struct DefGenericPenaltyOpts<'a> {
    pub serial: bool,
    pub description: &'a str,
    pub score: i32,
    pub connector: &'a str,
}

pub fn def_generic_penalty_body(
    segment_list_left: &KaniLiteSegmentList,
    segment_list_right: &KaniLiteSegmentList,
    test_left: impl Fn(&KaniLiteSegmentList) -> bool,
    test_right: impl Fn(&KaniLiteSegmentList) -> bool,
    opts: &DefGenericPenaltyOpts<'_>,
) -> Option<Synergy> {
    let start = segment_list_left.end;
    let end = segment_list_right.start;
    // dict-grammar.lisp:978-980 (and (if serial (= start end) t) (funcall test-left ...) (funcall test-right ...))
    if (!opts.serial || start == end)
        && test_left(segment_list_left)
        && test_right(segment_list_right)
    {
        // dict-grammar.lisp:981-984 (make-synergy :start :end :description :connector :score)
        Some(Synergy {
            description: Some(opts.description.to_string()),
            connector: Some(opts.connector.to_string()),
            score: opts.score,
            start,
            end,
        })
    } else {
        None
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod penalty_short_inner {
use super::{def_generic_penalty_body, DefGenericPenaltyOpts};
use crate::dict::grammar::filter::filter_short_kana;
use crate::dict::kani::KaniLiteSegmentList;
use crate::dict::grammar::synergy::Synergy;

pub fn penalty_short(l: &KaniLiteSegmentList, r: &KaniLiteSegmentList) -> Option<Synergy> {
    def_generic_penalty_body(
        l,
        r,
        filter_short_kana(1, vec![]),
        filter_short_kana(1, vec!["と".to_string()]),
        &DefGenericPenaltyOpts {
            serial: false,
            description: "short",
            score: -9,
            connector: " ",
        },
    )
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

    fn seg(start: usize, end: usize, kpcl: (bool, bool, bool, bool), text: &str) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(info_with(kpcl, vec![999])),
            top: None,
            text: Some(text.to_string()),
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_b.lisp on .103, 2026-05-18).

    #[test]
    fn d1_both_spans_one_returns_synergy() {
        let l = lite_sl_owned(0, 1, vec![seg(0, 1, (false, false, false, false), "あ")]);
        let r = lite_sl_owned(3, 4, vec![seg(3, 4, (false, false, false, false), "い")]);
        let got = penalty_short(&l, &r).expect("synergy");
        assert_eq!(got.description.as_deref(), Some("short"));
        assert_eq!(got.connector.as_deref(), Some(" "));
        assert_eq!(got.score, -9);
        assert_eq!(got.start, 1);
        assert_eq!(got.end, 3);
    }

    #[test]
    fn d2_l_span_two_returns_none() {
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, (false, false, false, false), "あい")]);
        let r = lite_sl_owned(2, 3, vec![seg(2, 3, (false, false, false, false), "い")]);
        assert!(penalty_short(&l, &r).is_none());
    }

    #[test]
    fn d3_r_text_in_r_except_returns_none() {
        let l = lite_sl_owned(0, 1, vec![seg(0, 1, (false, false, false, false), "あ")]);
        let r = lite_sl_owned(5, 6, vec![seg(5, 6, (false, false, false, false), "と")]);
        assert!(penalty_short(&l, &r).is_none());
    }

    #[test]
    fn d4_l_text_to_not_in_l_except_returns_synergy() {
        let l = lite_sl_owned(0, 1, vec![seg(0, 1, (false, false, false, false), "と")]);
        let r = lite_sl_owned(3, 4, vec![seg(3, 4, (false, false, false, false), "い")]);
        let got = penalty_short(&l, &r).expect("synergy");
        assert_eq!(got.score, -9);
    }

    #[test]
    fn d5_l_kpcl_first_set_returns_none() {
        let l = lite_sl_owned(0, 1, vec![seg(0, 1, (true, false, false, false), "あ")]);
        let r = lite_sl_owned(2, 3, vec![seg(2, 3, (false, false, false, false), "い")]);
        assert!(penalty_short(&l, &r).is_none());
    }

    #[test]
    fn d6_serial_nil_allows_non_adjacent() {
        let l = lite_sl_owned(0, 1, vec![seg(0, 1, (false, false, false, false), "あ")]);
        let r = lite_sl_owned(100, 101, vec![seg(100, 101, (false, false, false, false), "い")]);
        let got = penalty_short(&l, &r).expect("synergy");
        assert_eq!(got.start, 1);
        assert_eq!(got.end, 100);
    }

    #[test]
    fn d7_empty_l_segments_returns_none() {
        let l = lite_sl_owned(0, 1, vec![]);
        let r = lite_sl_owned(1, 2, vec![seg(1, 2, (false, false, false, false), "い")]);
        assert!(penalty_short(&l, &r).is_none());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod penalty_semi_final_inner {
use crate::dict::errata::semi_final_prt;
use super::{def_generic_penalty_body, DefGenericPenaltyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::kani::KaniLiteSegmentList;
use crate::dict::grammar::synergy::Synergy;

pub fn penalty_semi_final(l: &KaniLiteSegmentList, r: &KaniLiteSegmentList) -> Option<Synergy> {
    let f = filter_in_seq_set(semi_final_prt().to_vec());
    def_generic_penalty_body(
        l,
        r,
        // dict-grammar.lisp:1004-1006 (test-left lambda over (apply 'filter-in-seq-set *semi-final-prt*))
        |sl| sl.segments.iter().any(|s| f(s)),
        // dict-grammar.lisp:1007 (test-right = (constantly t))
        |_| true,
        &DefGenericPenaltyOpts {
            serial: true,
            description: "semi-final not final",
            score: -15,
            connector: " ",
        },
    )
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

    fn info_with(seq_set: Vec<i32>) -> KaniSegmentInfo {
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
            info: Some(info_with(seq_set)),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_b.lisp on .103, 2026-05-18).

    #[test]
    fn b1_non_adjacent_returns_none() {
        let l = lite_sl_owned(0, 1, vec![seg(0, 1, vec![2029120])]);
        let r = lite_sl_owned(5, 6, vec![seg(5, 6, vec![999])]);
        assert!(penalty_semi_final(&l, &r).is_none());
    }

    #[test]
    fn b2_adjacent_with_semi_final_seq_returns_synergy() {
        let l = lite_sl_owned(0, 1, vec![seg(0, 1, vec![2029120])]);
        let r = lite_sl_owned(1, 2, vec![seg(1, 2, vec![999])]);
        let got = penalty_semi_final(&l, &r).expect("synergy");
        assert_eq!(got.description.as_deref(), Some("semi-final not final"));
        assert_eq!(got.connector.as_deref(), Some(" "));
        assert_eq!(got.score, -15);
        assert_eq!(got.start, 1);
        assert_eq!(got.end, 1);
    }

    #[test]
    fn b3_adjacent_no_semi_final_seq_returns_none() {
        let l = lite_sl_owned(0, 1, vec![seg(0, 1, vec![999])]);
        let r = lite_sl_owned(1, 2, vec![seg(1, 2, vec![888])]);
        assert!(penalty_semi_final(&l, &r).is_none());
    }

    #[test]
    fn b4_empty_l_segments_returns_none() {
        let l = lite_sl_owned(0, 1, vec![]);
        let r = lite_sl_owned(1, 2, vec![seg(1, 2, vec![888])]);
        assert!(penalty_semi_final(&l, &r).is_none());
    }

    #[test]
    fn b5_l_two_segs_one_matches_returns_synergy() {
        let l = lite_sl_owned(0, 1, vec![seg(0, 1, vec![999]), seg(0, 1, vec![2086640])]);
        let r = lite_sl_owned(1, 2, vec![seg(1, 2, vec![888])]);
        let got = penalty_semi_final(&l, &r).expect("synergy");
        assert_eq!(got.score, -15);
        assert_eq!(got.description.as_deref(), Some("semi-final not final"));
    }

    #[test]
    fn b6_start_end_carry_through() {
        let l = lite_sl_owned(1, 3, vec![seg(1, 3, vec![2029120])]);
        let r = lite_sl_owned(3, 4, vec![seg(3, 4, vec![999])]);
        let got = penalty_semi_final(&l, &r).expect("synergy");
        assert_eq!(got.start, 3);
        assert_eq!(got.end, 3);
        assert_eq!(got.score, -15);
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod get_penalties_inner {
use std::sync::Arc;

use super::PENALTY_LIST;
use crate::dict::kani::KaniLiteSegmentList;
use crate::dict::kani::KaniLitePathElement;

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
    use crate::dict::errata::semi_final_prt;
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

    fn unwrap_synergy(elem: &KaniLitePathElement) -> &crate::dict::grammar::synergy::Synergy {
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
}
