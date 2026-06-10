mod penalty_short {
    use crate::dict::conj::ConjData;
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

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
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        })
    }

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
        let r = lite_sl_owned(
            100,
            101,
            vec![seg(100, 101, (false, false, false, false), "い")],
        );
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

mod penalty_semi_final {
    use crate::dict::conj::ConjData;
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

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
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        })
    }

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

mod get_penalties {
    use crate::dict::conj::ConjData;
    use crate::dict::errata::semi_final_prt;
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

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
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
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

    #[test]
    fn a_no_penalty_returns_two_element_list() {
        let l = lite_sl(
            0,
            3,
            vec![seg(0, 3, (true, false, false, false), vec![999], "abc")],
        );
        let r = lite_sl(
            3,
            6,
            vec![seg(3, 6, (true, false, false, false), vec![888], "def")],
        );
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
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, (false, false, false, false), vec![999], "あ")],
        );
        let r = lite_sl(
            3,
            4,
            vec![seg(3, 4, (false, false, false, false), vec![888], "い")],
        );
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
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, (false, false, false, false), vec![semi_seq], "x")],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(1, 2, (false, false, false, false), vec![999], "y")],
        );
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
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, (false, false, false, false), vec![semi_seq], "x")],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(1, 2, (false, false, false, false), vec![999], "y")],
        );
        let res = get_penalties(&l, &r);
        assert_eq!(res.len(), 3);
        let syn = unwrap_synergy(&res[1]);
        assert_eq!(syn.description.as_deref(), Some("semi-final not final"));
        assert_eq!(syn.score, -15);
    }

    #[test]
    fn e_penalty_branch_arg_order_seg_right_then_seg_left() {
        let l = lite_sl(
            10,
            11,
            vec![seg(10, 11, (false, false, false, false), vec![444], "α")],
        );
        let r = lite_sl(
            13,
            14,
            vec![seg(13, 14, (false, false, false, false), vec![555], "β")],
        );
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
        let l = lite_sl(
            10,
            13,
            vec![seg(
                10,
                13,
                (true, false, false, false),
                vec![444],
                "α-long",
            )],
        );
        let r = lite_sl(
            13,
            16,
            vec![seg(
                13,
                16,
                (true, false, false, false),
                vec![555],
                "β-long",
            )],
        );
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
        let r = lite_sl(
            1,
            2,
            vec![seg(1, 2, (false, false, false, false), vec![999], "い")],
        );
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

mod classify {
    use crate::dict::grammar::segfilter::*;

    #[test]
    fn partitions_by_predicate_preserving_order() {
        let (yep, nope) = classify(|n: &i32| n % 2 != 0, &[1, 2, 3, 4, 5]);
        assert_eq!(yep, vec![1, 3, 5]);
        assert_eq!(nope, vec![2, 4]);
    }

    #[test]
    fn empty_input_yields_empty_outputs() {
        let (yep, nope) = classify(|n: &i32| n % 2 != 0, &[]);
        assert!(yep.is_empty());
        assert!(nope.is_empty());
    }

    #[test]
    fn all_nope_branch() {
        let (yep, nope) = classify(|n: &i32| n % 2 != 0, &[2, 4, 6]);
        assert!(yep.is_empty());
        assert_eq!(nope, vec![2, 4, 6]);
    }

    #[test]
    fn all_yep_branch() {
        let (yep, nope) = classify(|_n: &i32| true, &[1, 2, 3]);
        assert_eq!(yep, vec![1, 2, 3]);
        assert!(nope.is_empty());
    }
}

mod def_segfilter_must_follow_macro {
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;
    // Each test exercises one branch of the segfilter helper with
    // synthetic left/right segments, independent of any dictionary lookup.

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

    fn info(seq_set: Vec<i32>) -> KaniSegmentInfo {
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

    fn seg(start: usize, end: usize, seq_set: Vec<i32>) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(info(seq_set)),
            top: None,
            text: None,
        }
    }

    fn sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    /// `sat-r` empty → pass through `(l, r)` unchanged.
    #[test]
    fn clause_1_no_right_match_passes_through() {
        let r = sl(0, 1, vec![seg(0, 1, vec![999])]);
        let result =
            def_segfilter_must_follow_body(None, &r, |_| true, |s| s.seq_set.contains(&100), false);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    /// `allow_first && l=None` → pass through even when sat-r is full.
    #[test]
    fn clause_1_allow_first_passes_through_when_l_none() {
        let r = sl(0, 1, vec![seg(0, 1, vec![100])]);
        let result =
            def_segfilter_must_follow_body(None, &r, |_| true, |s| s.seq_set.contains(&100), true);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    /// `l=None` without `allow_first`, `con_r` empty → empty result.
    #[test]
    fn clause_2_l_none_all_right_matches_returns_empty() {
        let r = sl(0, 1, vec![seg(0, 1, vec![100])]);
        let result =
            def_segfilter_must_follow_body(None, &r, |_| true, |s| s.seq_set.contains(&100), false);
        assert!(result.is_empty());
    }

    /// `l=None` without `allow_first`, `con_r` non-empty → drop matching segs.
    #[test]
    fn clause_2_l_none_mixed_right_drops_matches() {
        let r = sl(0, 1, vec![seg(0, 1, vec![100]), seg(0, 1, vec![999])]);
        let result =
            def_segfilter_must_follow_body(None, &r, |_| true, |s| s.seq_set.contains(&100), false);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    /// Gap (l.end ≠ r.start) with `con_r` empty → empty result.
    #[test]
    fn clause_2_gap_all_right_matches_returns_empty() {
        let l = sl(0, 1, vec![seg(0, 1, vec![999])]);
        let r = sl(2, 3, vec![seg(2, 3, vec![100])]);
        let result = def_segfilter_must_follow_body(
            Some(&l),
            &r,
            |_| true,
            |s| s.seq_set.contains(&100),
            true,
        );
        assert!(result.is_empty());
    }

    /// T-branch with `con_l` empty → pass through `(l, r)` unchanged.
    #[test]
    fn t_branch_all_left_satisfies_passes_through() {
        let l = sl(0, 1, vec![seg(0, 1, vec![999])]);
        let r = sl(1, 2, vec![seg(1, 2, vec![100])]);
        let result = def_segfilter_must_follow_body(
            Some(&l),
            &r,
            |_| true,
            |s| s.seq_set.contains(&100),
            false,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    /// T-branch with `sat_l` non-empty and `con_r` non-empty → two pairs,
    /// `(sat_l, sat_r)` prepended.
    #[test]
    fn t_branch_mixed_both_emits_two_pairs() {
        let l = sl(0, 1, vec![seg(0, 1, vec![100]), seg(0, 1, vec![999])]);
        let r = sl(1, 2, vec![seg(1, 2, vec![100]), seg(1, 2, vec![999])]);
        let result = def_segfilter_must_follow_body(
            Some(&l),
            &r,
            |s| s.seq_set.contains(&100),
            |s| s.seq_set.contains(&100),
            false,
        );
        assert_eq!(result.len(), 2);
        // First pair: sat_l × sat_r (prepended).
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![100]);
        assert_eq!(result[0].1.segments[0].seq_set, vec![100]);
        // Second pair: l unchanged × con_r.
        assert_eq!(result[1].0.as_ref().unwrap().segments.len(), 2);
        assert_eq!(result[1].1.segments[0].seq_set, vec![999]);
    }

    /// T-branch with `sat_l` empty and `con_r` non-empty → only the
    /// base pair (no prepended sat-pair).
    #[test]
    fn t_branch_no_left_satisfies_emits_base_only() {
        let l = sl(0, 1, vec![seg(0, 1, vec![999])]);
        let r = sl(1, 2, vec![seg(1, 2, vec![100]), seg(1, 2, vec![999])]);
        let result = def_segfilter_must_follow_body(
            Some(&l),
            &r,
            |_| false,
            |s| s.seq_set.contains(&100),
            false,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    /// T-branch with `sat_l` non-empty and `con_r` empty → only the
    /// prepended sat-pair.
    #[test]
    fn t_branch_no_right_contradicts_emits_sat_only() {
        let l = sl(0, 1, vec![seg(0, 1, vec![100]), seg(0, 1, vec![999])]);
        let r = sl(1, 2, vec![seg(1, 2, vec![100])]);
        let result = def_segfilter_must_follow_body(
            Some(&l),
            &r,
            |s| s.seq_set.contains(&100),
            |s| s.seq_set.contains(&100),
            false,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![100]);
        assert_eq!(result[0].1.segments[0].seq_set, vec![100]);
    }
}

mod segfilter_aux_verb {
    use crate::dict::conj::ConjData;
    use crate::dict::dao::ConjProp;
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

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
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    #[test]
    fn a_l_nil_r_no_match() {
        // No left, right has no aux-verb match: right passes through unchanged.
        let r = lite_sl(0, 2, vec![seg(0, 2, info_with_seq_set(vec![999]))]);
        let result = segfilter_aux_verb(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn b_l_nil_r_all_match() {
        // No left, every right segment is an aux verb: result is empty.
        let r = lite_sl(0, 2, vec![seg(0, 2, info_with_seq_set(vec![1342560]))]);
        let result = segfilter_aux_verb(None, &r);
        assert!(result.is_empty());
    }

    #[test]
    fn c_l_nil_r_mixed() {
        // No left, right is mixed: only the non-aux-verb segment survives.
        let r = lite_sl(
            0,
            2,
            vec![
                seg(0, 2, info_with_seq_set(vec![1342560])),
                seg(0, 2, info_with_seq_set(vec![999])),
            ],
        );
        let result = segfilter_aux_verb(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn d_l_adj_gap_r_mixed() {
        // Left not adjacent to right, right mixed: left unchanged, right
        // reduced to the non-aux-verb segment.
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_conj(vec![]))]);
        let r = lite_sl(
            2,
            4,
            vec![
                seg(2, 4, info_with_seq_set(vec![1342560])),
                seg(2, 4, info_with_seq_set(vec![999])),
            ],
        );
        let result = segfilter_aux_verb(Some(&l), &r);
        assert_eq!(result.len(), 1);
        let (lp, rp) = &result[0];
        let lp_ref = lp.as_ref().unwrap();
        assert_eq!(lp_ref.segments.len(), 1);
        assert_eq!(rp.segments.len(), 1);
        assert_eq!(rp.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn e_l_no_sat_r_mixed() {
        // Left has no qualifying conjugation, right mixed: left unchanged,
        // right reduced to the non-aux-verb segment.
        let l = lite_sl(0, 2, vec![seg(0, 2, info_with_conj(vec![]))]);
        let r = lite_sl(
            2,
            4,
            vec![
                seg(2, 4, info_with_seq_set(vec![1342560])),
                seg(2, 4, info_with_seq_set(vec![999])),
            ],
        );
        let result = segfilter_aux_verb(Some(&l), &r);
        assert_eq!(result.len(), 1);
        let (lp, rp) = &result[0];
        assert_eq!(lp.as_ref().unwrap().segments.len(), 1);
        assert_eq!(rp.segments.len(), 1);
        assert_eq!(rp.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn f_l_mixed_r_mixed() {
        // Both sides mixed: two splits result — the qualifying-left ×
        // aux-verb-right pair, and the full-left × remaining-right pair.
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
        let result = segfilter_aux_verb(Some(&l), &r);
        assert_eq!(result.len(), 2);

        // First pair: qualifying left × aux-verb right.
        let (lp0, rp0) = &result[0];
        let lp0_ref = lp0.as_ref().unwrap();
        assert_eq!(lp0_ref.segments.len(), 1);
        assert_eq!(rp0.segments.len(), 1);
        assert_eq!(rp0.segments[0].seq_set, vec![1342560]);

        // Second pair: full left × remaining right.
        let (lp1, rp1) = &result[1];
        let lp1_ref = lp1.as_ref().unwrap();
        assert_eq!(lp1_ref.segments.len(), 2);
        assert_eq!(rp1.segments.len(), 1);
        assert_eq!(rp1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn g_l_all_sat_r_all_sat() {
        // Whole left qualifies and whole right is an aux verb: both pass
        // through unchanged.
        let l = lite_sl(0, 2, vec![seg(0, 2, info_with_conj(vec![cdata(13)]))]);
        let r = lite_sl(2, 4, vec![seg(2, 4, info_with_seq_set(vec![1342560]))]);
        let result = segfilter_aux_verb(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn i_l_all_sat_r_no_match() {
        // Whole left qualifies but right has no aux verb: both pass through
        // unchanged.
        let l = lite_sl(0, 2, vec![seg(0, 2, info_with_conj(vec![cdata(13)]))]);
        let r = lite_sl(2, 4, vec![seg(2, 4, info_with_seq_set(vec![999]))]);
        let result = segfilter_aux_verb(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn j_l_mixed_r_all_sat() {
        // Left mixed, whole right is an aux verb: only the qualifying-left ×
        // right pair, no full-left pair.
        let l = lite_sl(
            0,
            2,
            vec![
                seg(0, 2, info_with_conj(vec![cdata(13)])),
                seg(0, 2, info_with_conj(vec![cdata(3)])),
            ],
        );
        let r = lite_sl(2, 4, vec![seg(2, 4, info_with_seq_set(vec![1342560]))]);
        let result = segfilter_aux_verb(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![1342560]);
    }
}

mod segfilter_tsu_iru {
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

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
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

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

mod segfilter_n {
    use crate::dict::text_classes::{CompoundText, ScoreMod};
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

    fn kana(seq: i32) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: "x".into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn simple_word(seq: i32) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(kana(seq))
    }

    fn compound_word(child_seqs: &[i32]) -> KaniWordDispatchEnum {
        let words: Vec<KaniWordDispatchEnum> = child_seqs
            .iter()
            .map(|s| KaniWordDispatchEnum::Kana(kana(*s)))
            .collect();
        let primary = Box::new(words[0].clone());
        KaniWordDispatchEnum::Compound(CompoundText {
            text: String::new(),
            kana: String::new(),
            primary,
            words,
            score_base: None,
            score_mod: ScoreMod::Single(0),
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

    fn seg(start: usize, end: usize, word: KaniWordDispatchEnum, info: KaniSegmentInfo) -> Segment {
        Segment {
            start,
            end,
            word,
            score: None,
            info: Some(info),
            top: None,
            text: None,
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    #[test]
    fn n_a_l_nil_r_all_n_pass_through() {
        // No left, whole right is a noun: right passes through unchanged.
        let r = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                simple_word(2139720),
                info_with_seq_set(vec![2139720]),
            )],
        );
        let result = segfilter_n(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn n_b_l_nil_r_no_match() {
        // No left, right is not a noun: right passes through unchanged.
        let r = lite_sl(
            0,
            1,
            vec![seg(0, 1, simple_word(999), info_with_seq_set(vec![999]))],
        );
        let result = segfilter_n(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn n_c_l_nil_r_mixed_pass_through() {
        // No left, right is mixed: both segments pass through.
        let r = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, simple_word(2139720), info_with_seq_set(vec![2139720])),
                seg(0, 1, simple_word(999), info_with_seq_set(vec![999])),
            ],
        );
        let result = segfilter_n(None, &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.segments.len(), 2);
    }

    #[test]
    fn n_d_l_not_noun_r_n() {
        // Left is not a noun particle, right is a noun: both pass through.
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, simple_word(999), info_with_seq_set(vec![999]))],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(
                1,
                2,
                simple_word(2139720),
                info_with_seq_set(vec![2139720]),
            )],
        );
        let result = segfilter_n(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn n_e_l_is_noun_r_n_empty() {
        // Left is the noun particle は and right is a noun: result is empty.
        let l = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                simple_word(2028920),
                info_with_seq_set(vec![2028920]),
            )],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(
                1,
                2,
                simple_word(2139720),
                info_with_seq_set(vec![2139720]),
            )],
        );
        let result = segfilter_n(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn n_f_l_is_noun_r_mixed() {
        // Left is a noun particle, right mixed: left unchanged, right reduced
        // to the non-noun segment.
        let l = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                simple_word(2028920),
                info_with_seq_set(vec![2028920]),
            )],
        );
        let r = lite_sl(
            1,
            2,
            vec![
                seg(1, 2, simple_word(2139720), info_with_seq_set(vec![2139720])),
                seg(1, 2, simple_word(999), info_with_seq_set(vec![999])),
            ],
        );
        let result = segfilter_n(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(
            result[0].0.as_ref().unwrap().segments[0].seq_set,
            vec![2028920]
        );
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn n_g_l_mixed_r_all_n() {
        // Left mixes a noun particle with a non-noun, right is all noun: only
        // the non-noun left × noun right pair survives.
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, simple_word(2028920), info_with_seq_set(vec![2028920])),
                seg(0, 1, simple_word(999), info_with_seq_set(vec![999])),
            ],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(
                1,
                2,
                simple_word(2139720),
                info_with_seq_set(vec![2139720]),
            )],
        );
        let result = segfilter_n(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![999]);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn n_h_gap_r_mixed() {
        // Left not adjacent to right, right mixed: right reduced to the
        // non-noun segment.
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, simple_word(999), info_with_seq_set(vec![999]))],
        );
        let r = lite_sl(
            2,
            3,
            vec![
                seg(2, 3, simple_word(2139720), info_with_seq_set(vec![2139720])),
                seg(2, 3, simple_word(999), info_with_seq_set(vec![999])),
            ],
        );
        let result = segfilter_n(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn n_i_gap_r_all_n_empty() {
        // Left not adjacent to right, whole right is a noun: result is empty.
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, simple_word(999), info_with_seq_set(vec![999]))],
        );
        let r = lite_sl(
            2,
            3,
            vec![seg(
                2,
                3,
                simple_word(2139720),
                info_with_seq_set(vec![2139720]),
            )],
        );
        let result = segfilter_n(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn n_j_l_compound_r_all_n() {
        // A compound left is never treated as a noun particle, so it passes
        // through together with the noun right.
        let lseg = Segment {
            start: 0,
            end: 2,
            word: compound_word(&[2028920, 999]),
            score: None,
            info: Some(info_with_seq_set(vec![2028920, 999])),
            top: None,
            text: None,
        };
        let l = lite_sl(0, 2, vec![lseg]);
        let r = lite_sl(
            2,
            3,
            vec![seg(
                2,
                3,
                simple_word(2139720),
                info_with_seq_set(vec![2139720]),
            )],
        );
        let result = segfilter_n(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }
}

mod segfilter_wokarasu {
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

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
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    #[test]
    fn w_a_l_nil_r_karasu_empty() {
        // No left, whole right is からす: result is empty.
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2087020]))]);
        let result = segfilter_wokarasu(None, &r);
        assert!(result.is_empty());
    }

    #[test]
    fn w_b_l_nil_r_mixed() {
        // No left, right mixed: only the non-からす segment survives.
        let r = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, info_with_seq_set(vec![2087020])),
                seg(0, 1, info_with_seq_set(vec![999])),
            ],
        );
        let result = segfilter_wokarasu(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn w_c_l_nil_r_no_match() {
        // No left, right has no からす match: right passes through.
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let result = segfilter_wokarasu(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn w_d_l_wo_r_karasu_pass_through() {
        // Left を, right からす: both pass through.
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2029010]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![2087020]))]);
        let result = segfilter_wokarasu(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn w_e_l_not_wo_r_karasu_empty() {
        // Left is not を, right is からす: result is empty.
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![2087020]))]);
        let result = segfilter_wokarasu(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn w_f_l_mixed_r_mixed_two_pairs() {
        // Both sides mix を/からす with other words: two pairs result — the
        // を × からす pair, and the full-left × remaining-right pair.
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, info_with_seq_set(vec![2029010])),
                seg(0, 1, info_with_seq_set(vec![999])),
            ],
        );
        let r = lite_sl(
            1,
            2,
            vec![
                seg(1, 2, info_with_seq_set(vec![2087020])),
                seg(1, 2, info_with_seq_set(vec![888])),
            ],
        );
        let result = segfilter_wokarasu(Some(&l), &r);
        assert_eq!(result.len(), 2);

        // First pair: を left × からす right.
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(
            result[0].0.as_ref().unwrap().segments[0].seq_set,
            vec![2029010]
        );
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![2087020]);

        // Second pair: full left × remaining right.
        assert_eq!(result[1].0.as_ref().unwrap().segments.len(), 2);
        assert_eq!(result[1].1.segments.len(), 1);
        assert_eq!(result[1].1.segments[0].seq_set, vec![888]);
    }

    #[test]
    fn w_g_gap_r_mixed() {
        // Left not adjacent to right, right mixed: right reduced to the
        // non-からす segment.
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2029010]))]);
        let r = lite_sl(
            2,
            3,
            vec![
                seg(2, 3, info_with_seq_set(vec![2087020])),
                seg(2, 3, info_with_seq_set(vec![888])),
            ],
        );
        let result = segfilter_wokarasu(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![888]);
    }

    #[test]
    fn w_h_gap_r_all_karasu_empty() {
        // Left not adjacent to right, whole right is からす: result is empty.
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2029010]))]);
        let r = lite_sl(2, 3, vec![seg(2, 3, info_with_seq_set(vec![2087020]))]);
        let result = segfilter_wokarasu(Some(&l), &r);
        assert!(result.is_empty());
    }
}

mod segfilter_badend {
    use crate::dict::text_classes::{CompoundText, ScoreMod};
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::Segment;
    use crate::dict::dao::SimpleText;

    fn kana(text: &str, seq: i32) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn compound(child_texts: &[&str]) -> KaniWordDispatchEnum {
        let words: Vec<KaniWordDispatchEnum> = child_texts
            .iter()
            .enumerate()
            .map(|(i, t)| KaniWordDispatchEnum::Kana(kana(t, 9900 + i as i32)))
            .collect();
        let primary = Box::new(words[0].clone());
        KaniWordDispatchEnum::Compound(CompoundText {
            text: String::new(),
            kana: String::new(),
            primary,
            words,
            score_base: None,
            score_mod: ScoreMod::Single(0),
        })
    }

    fn seg(start: usize, end: usize, word: KaniWordDispatchEnum) -> Segment {
        Segment {
            start,
            end,
            word,
            score: None,
            info: None,
            top: None,
            text: None,
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    #[test]
    fn ba_a_l_nil_r_all_match_returns_empty() {
        // No left, every right segment is a bad ending: result is empty.
        let seg_chai = seg(1, 2, compound(&["ちゃい"]));
        let r = lite_sl(1, 2, vec![seg_chai]);
        let result = segfilter_badend(None, &r);
        assert!(result.is_empty());
    }

    #[test]
    fn ba_b_l_nil_r_mixed() {
        // No left, right mixed: only the non-bad-ending segment survives.
        let seg_chai = seg(1, 2, compound(&["ちゃい"]));
        let seg_x = seg(1, 2, compound(&["x"]));
        let r = lite_sl(1, 2, vec![seg_chai, seg_x]);
        let result = segfilter_badend(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn ba_c_l_nil_r_no_match() {
        // No left, right has no bad ending: right passes through.
        let seg_x = seg(1, 2, compound(&["x"]));
        let r = lite_sl(1, 2, vec![seg_x]);
        let result = segfilter_badend(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn ba_d_l_adj_r_mixed_emits_base_pair_only() {
        // The left is never split (it has no qualifying half), so an adjacent
        // left with a mixed right yields only the left × non-bad-ending pair.
        let seg_simp = seg(0, 1, KaniWordDispatchEnum::Kana(kana("い", 9995)));
        let seg_chai = seg(1, 3, compound(&["ちゃい"]));
        let seg_x = seg(1, 3, compound(&["x"]));
        let l = lite_sl(0, 1, vec![seg_simp]);
        let r = lite_sl(1, 3, vec![seg_chai, seg_x]);
        let result = segfilter_badend(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn ba_e_l_adj_r_all_match_empty_result() {
        // Adjacent left, whole right is a bad ending: result is empty.
        let seg_simp = seg(0, 1, KaniWordDispatchEnum::Kana(kana("い", 9995)));
        let seg_chai = seg(1, 3, compound(&["ちゃい"]));
        let l = lite_sl(0, 1, vec![seg_simp]);
        let r = lite_sl(1, 3, vec![seg_chai]);
        let result = segfilter_badend(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn ba_f_l_adj_gap_r_mixed() {
        // Left not adjacent to right, right mixed: left unchanged, right
        // reduced to the non-bad-ending segment.
        let seg_simp = seg(0, 1, KaniWordDispatchEnum::Kana(kana("い", 9995)));
        let seg_chai = seg(2, 4, compound(&["ちゃい"]));
        let seg_x = seg(2, 4, compound(&["x"]));
        let l = lite_sl(0, 1, vec![seg_simp]);
        let r = lite_sl(2, 4, vec![seg_chai, seg_x]);
        let result = segfilter_badend(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn ba_g_l_adj_r_no_match() {
        // Adjacent left, right has no bad ending: both pass through.
        let seg_simp = seg(0, 1, KaniWordDispatchEnum::Kana(kana("い", 9995)));
        let seg_x = seg(1, 3, compound(&["x"]));
        let l = lite_sl(0, 1, vec![seg_simp]);
        let r = lite_sl(1, 3, vec![seg_x]);
        let result = segfilter_badend(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }
}

mod segfilter_sukiyoki {
    use crate::dict::conj::ConjData;
    use crate::dict::dao::ConjProp;
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

    fn kana(text: &str, seq: i32) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn cdata_54() -> ConjData {
        ConjData {
            seq: Some(1),
            from: Some(2),
            via: None,
            prop: Some(ConjProp {
                id: 0,
                conj_id: 0,
                conj_type: 54,
                pos: String::new(),
                neg: None,
                fml: None,
            }),
            src_map: vec![],
        }
    }

    fn info(seq_set: Vec<i32>, conj: Vec<ConjData>) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: vec![],
            seq_set,
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

    fn seg(start: usize, end: usize, t: &str, seq: i32, info: Option<KaniSegmentInfo>) -> Segment {
        Segment {
            start,
            end,
            word: KaniWordDispatchEnum::Kana(kana(t, seq)),
            score: None,
            info,
            top: None,
            text: None,
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    #[test]
    fn sk_a_l_nil_r_suki_conj54_empty() {
        // No left, right is the matching 好き conjugation: result is empty.
        let r = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                "好き",
                100,
                Some(info(vec![100], vec![cdata_54()])),
            )],
        );
        let result = segfilter_sukiyoki(None, &r);
        assert!(result.is_empty());
    }

    #[test]
    fn sk_b_l_nil_r_mixed() {
        // No left, right mixed: only the non-matching segment survives.
        let r = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, "好き", 100, Some(info(vec![100], vec![cdata_54()]))),
                seg(0, 1, "abc", 999, Some(info(vec![999], vec![]))),
            ],
        );
        let result = segfilter_sukiyoki(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
        match &result[0].1.segments[0].source.word {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, 999);
                assert_eq!(k.text, "abc");
            }
            _ => panic!("expected Kana variant"),
        }
    }

    #[test]
    fn sk_c_l_nil_r_no_match() {
        // No left, right has no match: right passes through.
        let r = lite_sl(
            0,
            1,
            vec![seg(0, 1, "abc", 999, Some(info(vec![999], vec![])))],
        );
        let result = segfilter_sukiyoki(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn sk_d_l_simple_r_suki_empty() {
        // Plain left adjacent to a matching 好き right: result is empty.
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, "abc", 999, Some(info(vec![999], vec![])))],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(
                1,
                2,
                "好き",
                100,
                Some(info(vec![100], vec![cdata_54()])),
            )],
        );
        let result = segfilter_sukiyoki(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn sk_e_l_simple_r_mixed_base_only() {
        // Plain left, right mixed: left unchanged, right reduced to the
        // non-matching segment.
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, "abc", 999, Some(info(vec![999], vec![])))],
        );
        let r = lite_sl(
            1,
            2,
            vec![
                seg(1, 2, "好き", 100, Some(info(vec![100], vec![cdata_54()]))),
                seg(1, 2, "abc", 999, Some(info(vec![999], vec![]))),
            ],
        );
        let result = segfilter_sukiyoki(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        match &result[0].1.segments[0].source.word {
            KaniWordDispatchEnum::Kana(k) => assert_eq!(k.seq, 999),
            _ => panic!("expected Kana variant"),
        }
    }

    #[test]
    fn sk_f_gap_r_suki_empty() {
        // Left not adjacent to right, whole right is 好き: result is empty.
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, "abc", 999, Some(info(vec![999], vec![])))],
        );
        let r = lite_sl(
            2,
            3,
            vec![seg(
                2,
                3,
                "好き",
                100,
                Some(info(vec![100], vec![cdata_54()])),
            )],
        );
        let result = segfilter_sukiyoki(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn sk_g_l_nil_r_suki_no_conj_pass_through() {
        // 好き text without the required conjugation does not match, so the
        // right passes through.
        let r = lite_sl(
            0,
            1,
            vec![seg(0, 1, "好き", 100, Some(info(vec![100], vec![])))],
        );
        let result = segfilter_sukiyoki(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn sk_h_l_nil_r_conj54_not_suki_pass_through() {
        // The right conjugation without text ending in 好き does not match, so
        // the right passes through.
        let r = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                "abc",
                100,
                Some(info(vec![100], vec![cdata_54()])),
            )],
        );
        let result = segfilter_sukiyoki(None, &r);
        assert_eq!(result.len(), 1);
    }
}

mod segfilter_roku {
    use crate::dict::text_classes::{CompoundText, ScoreMod};
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::Segment;
    use crate::dict::dao::SimpleText;

    fn kana(text: &str, seq: i32) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn simple_seg(start: usize, end: usize, t: &str, seq: i32) -> Segment {
        Segment {
            start,
            end,
            word: KaniWordDispatchEnum::Kana(kana(t, seq)),
            score: None,
            info: None,
            top: None,
            text: None,
        }
    }

    fn compound_ending_seg(start: usize, end: usize, inner_text: &str, inner_seq: i32) -> Segment {
        let words = vec![
            KaniWordDispatchEnum::Kana(kana("z", 11111)),
            KaniWordDispatchEnum::Kana(kana(inner_text, inner_seq)),
        ];
        let primary = Box::new(words[0].clone());
        let c = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary,
            words,
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        Segment {
            start,
            end,
            word: KaniWordDispatchEnum::Compound(c),
            score: None,
            info: None,
            top: None,
            text: None,
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    #[test]
    fn r_a_l_nil_r_ku_pass_through() {
        // No left, right is くる: right passes through.
        let r = lite_sl(0, 1, vec![simple_seg(0, 1, "くる", 100)]);
        let result = segfilter_roku(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn r_b_l_nil_r_not_ku() {
        // No left, right is not くる: right passes through.
        let r = lite_sl(0, 1, vec![simple_seg(0, 1, "あさ", 100)]);
        let result = segfilter_roku(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn r_c_l_simple_r_ku_pass_through() {
        // Plain left, right くる: both pass through.
        let l = lite_sl(0, 1, vec![simple_seg(0, 1, "abc", 999)]);
        let r = lite_sl(1, 2, vec![simple_seg(1, 2, "くる", 100)]);
        let result = segfilter_roku(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn r_d_l_iro_r_ku_empty() {
        // Left ends in いろ, right is くる: result is empty.
        let l = lite_sl(0, 2, vec![compound_ending_seg(0, 2, "いろ", 50)]);
        let r = lite_sl(2, 3, vec![simple_seg(2, 3, "くる", 100)]);
        let result = segfilter_roku(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn r_e_l_mixed_iro_r_ku_sat_push() {
        // Left mixes an いろ-ending compound with a plain word, right is くる:
        // only the plain left × くる pair survives.
        let l = lite_sl(
            0,
            2,
            vec![
                compound_ending_seg(0, 2, "いろ", 50),
                simple_seg(0, 2, "abc", 999),
            ],
        );
        let r = lite_sl(2, 3, vec![simple_seg(2, 3, "くる", 100)]);
        let result = segfilter_roku(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        // Surviving L seg is the simple one (text="abc", seq=999).
        match &result[0].0.as_ref().unwrap().segments[0].source.word {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "abc");
                assert_eq!(k.seq, 999);
            }
            _ => panic!("expected Kana variant"),
        }
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn r_f_gap_r_mixed() {
        // Left not adjacent to right, right mixed: only the non-くる reading
        // survives.
        let l = lite_sl(0, 1, vec![simple_seg(0, 1, "abc", 999)]);
        let r = lite_sl(
            2,
            3,
            vec![simple_seg(2, 3, "くる", 100), simple_seg(2, 3, "あさ", 999)],
        );
        let result = segfilter_roku(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        match &result[0].1.segments[0].source.word {
            KaniWordDispatchEnum::Kana(k) => assert_eq!(k.text, "あさ"),
            _ => panic!("expected Kana variant"),
        }
    }
}

mod segfilter_sae {
    use crate::dict::text_classes::{CompoundText, ScoreMod};
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

    fn kana(text: &str, seq: i32) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
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

    fn simple_seg(
        start: usize,
        end: usize,
        t: &str,
        seq: i32,
        info: Option<KaniSegmentInfo>,
    ) -> Segment {
        Segment {
            start,
            end,
            word: KaniWordDispatchEnum::Kana(kana(t, seq)),
            score: None,
            info,
            top: None,
            text: None,
        }
    }

    fn compound_ending_seg(start: usize, end: usize, last_seq: i32) -> Segment {
        let words = vec![
            KaniWordDispatchEnum::Kana(kana("a", 999)),
            KaniWordDispatchEnum::Kana(kana("b", last_seq)),
        ];
        let primary = Box::new(words[0].clone());
        let c = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary,
            words,
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        Segment {
            start,
            end,
            word: KaniWordDispatchEnum::Compound(c),
            score: None,
            info: None,
            top: None,
            text: None,
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    #[test]
    fn s_a_l_nil_r_e_pass_through() {
        // No left, right is える: right passes through.
        let r = lite_sl(0, 1, vec![simple_seg(0, 1, "える", 100, None)]);
        let result = segfilter_sae(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn s_b_l_nil_r_not_e() {
        // No left, right is not える: right passes through.
        let r = lite_sl(0, 1, vec![simple_seg(0, 1, "abc", 100, None)]);
        let result = segfilter_sae(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn s_c_l_simple_r_e() {
        // Plain left, right える: both pass through.
        let l = lite_sl(
            0,
            1,
            vec![simple_seg(
                0,
                1,
                "abc",
                999,
                Some(info_with_seq_set(vec![999])),
            )],
        );
        let r = lite_sl(1, 2, vec![simple_seg(1, 2, "える", 100, None)]);
        let result = segfilter_sae(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn s_d_l_compound_end_sae_r_e_empty() {
        // Left is a compound ending in さえ, right is える: result is empty.
        let l = lite_sl(0, 2, vec![compound_ending_seg(0, 2, 2029120)]);
        let r = lite_sl(2, 3, vec![simple_seg(2, 3, "える", 100, None)]);
        let result = segfilter_sae(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn s_e_l_mixed_r_e_sat_push() {
        // Left mixes a さえ-ending compound with a plain word, right is える:
        // only the plain left × える pair survives.
        let l = lite_sl(
            0,
            2,
            vec![
                compound_ending_seg(0, 2, 2029120),
                simple_seg(0, 2, "abc", 999, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let r = lite_sl(2, 3, vec![simple_seg(2, 3, "える", 100, None)]);
        let result = segfilter_sae(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        match &result[0].0.as_ref().unwrap().segments[0].source.word {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "abc");
                assert_eq!(k.seq, 999);
            }
            _ => panic!("expected simple Kana variant"),
        }
        assert_eq!(result[0].1.segments.len(), 1);
    }
}

mod segfilter_janai {
    use crate::dict::text_classes::{CompoundText, ScoreMod};
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

    fn kana(text: &str, seq: i32) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn simple_word(seq: i32) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(kana("x", seq))
    }

    fn compound_word_ending_in(seq: i32) -> KaniWordDispatchEnum {
        let words = vec![
            KaniWordDispatchEnum::Kana(kana("a", 999)),
            KaniWordDispatchEnum::Kana(kana("b", seq)),
        ];
        let primary = Box::new(words[0].clone());
        KaniWordDispatchEnum::Compound(CompoundText {
            text: String::new(),
            kana: String::new(),
            primary,
            words,
            score_base: None,
            score_mod: ScoreMod::Single(0),
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

    fn seg(
        start: usize,
        end: usize,
        word: KaniWordDispatchEnum,
        info: Option<KaniSegmentInfo>,
    ) -> Segment {
        Segment {
            start,
            end,
            word,
            score: None,
            info,
            top: None,
            text: None,
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    #[test]
    fn j_a_l_nil_r_janai_pass_through() {
        // No left, right is じゃない: right passes through.
        let r = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                simple_word(1529520),
                Some(info_with_seq_set(vec![1529520])),
            )],
        );
        let result = segfilter_janai(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn j_b_l_simple_r_janai_pass_through() {
        // A simple left is never a compound ending in は, so it and the
        // じゃない right both pass through.
        let l = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                simple_word(999),
                Some(info_with_seq_set(vec![999])),
            )],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(
                1,
                2,
                simple_word(1529520),
                Some(info_with_seq_set(vec![1529520])),
            )],
        );
        let result = segfilter_janai(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn j_c_l_compound_ending_ha_r_janai_empty() {
        // A compound left ending in は disqualifies it; against a じゃない
        // right the result is empty.
        let l = lite_sl(
            0,
            2,
            vec![seg(
                0,
                2,
                compound_word_ending_in(2028920),
                Some(info_with_seq_set(vec![2028920])),
            )],
        );
        let r = lite_sl(
            2,
            3,
            vec![seg(
                2,
                3,
                simple_word(1529520),
                Some(info_with_seq_set(vec![1529520])),
            )],
        );
        let result = segfilter_janai(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn j_d_l_mixed_compound_r_janai() {
        // Left mixes a は-ending compound with a simple word, right is
        // じゃない: only the simple left × right pair survives.
        let l = lite_sl(
            0,
            2,
            vec![
                seg(
                    0,
                    2,
                    compound_word_ending_in(2028920),
                    Some(info_with_seq_set(vec![2028920])),
                ),
                seg(0, 2, simple_word(999), Some(info_with_seq_set(vec![999]))),
            ],
        );
        let r = lite_sl(
            2,
            3,
            vec![seg(
                2,
                3,
                simple_word(1529520),
                Some(info_with_seq_set(vec![1529520])),
            )],
        );
        let result = segfilter_janai(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![999]);
    }

    #[test]
    fn j_e_gap_r_janai_mixed() {
        // Left not adjacent to right, right mixed: left unchanged, right
        // reduced to the non-じゃない segment.
        let l = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                simple_word(999),
                Some(info_with_seq_set(vec![999])),
            )],
        );
        let r = lite_sl(
            2,
            3,
            vec![
                seg(
                    2,
                    3,
                    simple_word(1296400),
                    Some(info_with_seq_set(vec![1296400])),
                ),
                seg(2, 3, simple_word(999), Some(info_with_seq_set(vec![999]))),
            ],
        );
        let result = segfilter_janai(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }
}

mod segfilter_nohayamete {
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

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
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    #[test]
    fn nh_a_l_nil_r_match() {
        // No left, right matches: right passes through.
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![1601080]))]);
        let result = segfilter_nohayamete(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn nh_b_l_nil_r_no_match() {
        // No left, right has no match: right passes through.
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let result = segfilter_nohayamete(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn nh_c_l_not_no_r_hayamete() {
        // Left is not の, right is はやめて: both pass through.
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1601080]))]);
        let result = segfilter_nohayamete(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn nh_d_l_is_no_r_hayamete_empty() {
        // Left is の, right is はやめて: result is empty.
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![1469800]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1601080]))]);
        let result = segfilter_nohayamete(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn nh_e_l_mixed_r_hayamete() {
        // Left mixes の with a non-の word, right is はやめて: only the
        // non-の left × right pair survives.
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, info_with_seq_set(vec![1469800])),
                seg(0, 1, info_with_seq_set(vec![999])),
            ],
        );
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1601080]))]);
        let result = segfilter_nohayamete(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![999]);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn nh_f_gap_r_mixed() {
        // Left not adjacent to right, right mixed: right reduced to the
        // non-はやめて segment.
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let r = lite_sl(
            2,
            3,
            vec![
                seg(2, 3, info_with_seq_set(vec![1601080])),
                seg(2, 3, info_with_seq_set(vec![999])),
            ],
        );
        let result = segfilter_nohayamete(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }
}

mod segfilter_toomou {
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

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
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    #[test]
    fn tm_a_l_nil_r_omou_pass_through() {
        // No left, right is おもう: right passes through.
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![1589350]))]);
        let result = segfilter_toomou(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn tm_b_l_nil_r_no_match() {
        // No left, right has no match: right passes through.
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let result = segfilter_toomou(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn tm_c_l_not_nandato_r_omou() {
        // Left is not なんだと, right is おもう: both pass through.
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1589350]))]);
        let result = segfilter_toomou(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn tm_d_l_nandato_r_omou_empty() {
        // Left is なんだと, right is おもう: result is empty.
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2837117]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1589350]))]);
        let result = segfilter_toomou(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn tm_e_l_mixed_r_omou() {
        // Left mixes なんだと with another word, right is おもう: only the
        // non-なんだと left × right pair survives.
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

mod segfilter_totte {
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

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
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    #[test]
    fn t_a_l_nil_r_totte_pass_through() {
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2086960]))]);
        let result = segfilter_totte(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn t_b_l_nil_r_no_match() {
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let result = segfilter_totte(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn t_c_l_not_to_r_totte() {
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![2086960]))]);
        let result = segfilter_totte(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn t_d_l_to_r_totte_empty() {
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![1008490]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![2086960]))]);
        let result = segfilter_totte(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn t_e_l_mixed_r_totte() {
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, info_with_seq_set(vec![1008490])),
                seg(0, 1, info_with_seq_set(vec![999])),
            ],
        );
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![2086960]))]);
        let result = segfilter_totte(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![999]);
        assert_eq!(result[0].1.segments.len(), 1);
    }
}

mod segfilter_dashi {
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

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

    fn seg(start: usize, end: usize, info: Option<KaniSegmentInfo>) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info,
            top: None,
            text: None,
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    #[test]
    fn da_a_l_nil_r_all_match_passes_through_allow_first() {
        // No left, whole right is する: right passes through.
        let r = lite_sl(
            0,
            1,
            vec![seg(0, 1, Some(info_with_seq_set(vec![1157170])))],
        );
        let result = segfilter_dashi(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn da_b_l_nil_r_mixed_passes_through() {
        // No left, right mixed: both segments pass through.
        let r = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![1157170]))),
                seg(0, 1, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let result = segfilter_dashi(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 2);
    }

    #[test]
    fn da_c_l_da_r_no_match_passes_through() {
        // Left is だ, right has no する match: both pass through.
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, Some(info_with_seq_set(vec![2089020])))],
        );
        let r = lite_sl(1, 3, vec![seg(1, 3, Some(info_with_seq_set(vec![999])))]);
        let result = segfilter_dashi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn da_d_l_sat_l_r_sat_r() {
        // Left has no だ and right is する: both pass through.
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![123])))]);
        let r = lite_sl(
            1,
            3,
            vec![seg(1, 3, Some(info_with_seq_set(vec![1157170])))],
        );
        let result = segfilter_dashi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn da_e_l_da_r_mixed() {
        // Left is only だ, right mixed: left unchanged, right reduced to the
        // non-する segment.
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, Some(info_with_seq_set(vec![2089020])))],
        );
        let r = lite_sl(
            1,
            3,
            vec![
                seg(1, 3, Some(info_with_seq_set(vec![1157170]))),
                seg(1, 3, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let result = segfilter_dashi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn da_f_l_de_r_suru() {
        // Left contains で, right is する: both pass through.
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, Some(info_with_seq_set(vec![2028980])))],
        );
        let r = lite_sl(
            1,
            3,
            vec![seg(1, 3, Some(info_with_seq_set(vec![1157170])))],
        );
        let result = segfilter_dashi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn da_g_l_da_then_da_de_r_suru() {
        // Left has a だ-only segment and a だ+で segment, right is する: only
        // the だ+で left × right pair survives.
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![2089020]))),
                seg(0, 1, Some(info_with_seq_set(vec![2089020, 2028980]))),
            ],
        );
        let r = lite_sl(
            1,
            3,
            vec![seg(1, 3, Some(info_with_seq_set(vec![1157170])))],
        );
        let result = segfilter_dashi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(
            result[0].0.as_ref().unwrap().segments[0].seq_set,
            vec![2089020, 2028980]
        );
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn da_h_l_da_r_mixed_gap() {
        // Left だ not adjacent to right, right mixed: left unchanged, right
        // reduced to the non-する segment.
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, Some(info_with_seq_set(vec![2089020])))],
        );
        let r = lite_sl(
            2,
            4,
            vec![
                seg(2, 4, Some(info_with_seq_set(vec![1157170]))),
                seg(2, 4, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let result = segfilter_dashi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn da_i_l_info_nil_r_suru() {
        // A left segment with no info (empty sequence set) is treated as
        // qualifying, so it and the する right both pass through.
        let l = lite_sl(0, 1, vec![seg(0, 1, None)]);
        let r = lite_sl(
            1,
            3,
            vec![seg(1, 3, Some(info_with_seq_set(vec![1157170])))],
        );
        let result = segfilter_dashi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert!(result[0].0.as_ref().unwrap().segments[0].seq_set.is_empty());
    }
}

mod segfilter_dekiru {
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

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

    fn seg(start: usize, end: usize, info: Option<KaniSegmentInfo>) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info,
            top: None,
            text: None,
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    #[test]
    fn de_a_l_nil_r_all_match() {
        // No left, whole right is 来る: right passes through.
        let r = lite_sl(
            0,
            1,
            vec![seg(0, 1, Some(info_with_seq_set(vec![2830009])))],
        );
        let result = segfilter_dekiru(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn de_b_l_nil_r_mixed() {
        // No left, right mixed: both segments pass through.
        let r = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![999]))),
                seg(0, 1, Some(info_with_seq_set(vec![2830009]))),
            ],
        );
        let result = segfilter_dekiru(None, &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.segments.len(), 2);
    }

    #[test]
    fn de_c_l_de_r_no_match() {
        // Left is 出, right is not 来る: both pass through.
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, Some(info_with_seq_set(vec![1896380])))],
        );
        let r = lite_sl(1, 2, vec![seg(1, 2, Some(info_with_seq_set(vec![999])))]);
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn de_d_l_sat_l_r_sat_r() {
        // Left is not 出 and right is 来る: both pass through.
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![123])))]);
        let r = lite_sl(
            1,
            2,
            vec![seg(1, 2, Some(info_with_seq_set(vec![2830009])))],
        );
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn de_e_l_de_r_mixed() {
        // Left is only 出, right mixed: left unchanged, right reduced to the
        // non-来る segment.
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, Some(info_with_seq_set(vec![1896380])))],
        );
        let r = lite_sl(
            1,
            2,
            vec![
                seg(1, 2, Some(info_with_seq_set(vec![2830009]))),
                seg(1, 2, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn de_f_l_mixed_r_mixed() {
        // Both sides mixed: two splits — the non-出 left × 来る right pair, and
        // the full-left × remaining-right pair.
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![1896380]))),
                seg(0, 1, Some(info_with_seq_set(vec![888]))),
            ],
        );
        let r = lite_sl(
            1,
            2,
            vec![
                seg(1, 2, Some(info_with_seq_set(vec![2830009]))),
                seg(1, 2, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 2);

        // First pair: non-出 left × 来る right.
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![888]);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![2830009]);

        // Second pair: full left × remaining right.
        assert_eq!(result[1].0.as_ref().unwrap().segments.len(), 2);
        assert_eq!(result[1].1.segments.len(), 1);
        assert_eq!(result[1].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn de_g_l_mixed_r_all_sat() {
        // Left mixed, whole right is 来る: only the non-出 left × right pair
        // survives.
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![1896380]))),
                seg(0, 1, Some(info_with_seq_set(vec![888]))),
            ],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(1, 2, Some(info_with_seq_set(vec![2830009])))],
        );
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![888]);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn de_h_l_info_nil_r_sat() {
        // A left segment with no info (empty sequence set) is treated as
        // qualifying, so it and the 来る right both pass through.
        let l = lite_sl(0, 1, vec![seg(0, 1, None)]);
        let r = lite_sl(
            1,
            2,
            vec![seg(1, 2, Some(info_with_seq_set(vec![2830009])))],
        );
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert!(result[0].0.as_ref().unwrap().segments[0].seq_set.is_empty());
    }
}

mod apply_segfilters {
    use crate::dict::conj::ConjData;
    use crate::dict::dao::ConjProp;
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_lite_segment::KaniLiteSegment;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

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
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    fn assert_seq_set_seg(seg: &Arc<KaniLiteSegment>, start: usize, end: usize, seq_set: &[i32]) {
        assert_eq!(seg.source.start, start);
        assert_eq!(seg.source.end, end);
        assert_eq!(seg.seq_set, seq_set);
        assert!(seg.conj_types.is_empty());
        assert_eq!(seg.pos, 0);
        assert_eq!(seg.kpcl, 0);
    }

    fn assert_conj_seg(seg: &Arc<KaniLiteSegment>, start: usize, end: usize, conj_type: i32) {
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

mod segfilter_honorific {
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

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

    fn seg(start: usize, end: usize, info: Option<KaniSegmentInfo>) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info,
            top: None,
            text: None,
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    #[test]
    fn h_a_l_nil_r_all_honor_empty() {
        // No left, whole right is honorific: result is empty.
        let r = lite_sl(
            0,
            1,
            vec![seg(0, 1, Some(info_with_seq_set(vec![1247260])))],
        );
        let result = segfilter_honorific(None, &r);
        assert!(result.is_empty());
    }

    #[test]
    fn h_b_l_nil_r_mixed() {
        // No left, right mixed: only the non-honorific segment survives.
        let r = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![1247260]))),
                seg(0, 1, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let result = segfilter_honorific(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn h_c_l_nil_r_no_match() {
        // No left, right has no honorific: right passes through.
        let r = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![999])))]);
        let result = segfilter_honorific(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn h_d_l_not_noun_r_honor() {
        // Left is not a noun particle, right is honorific: both pass through.
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![999])))]);
        let r = lite_sl(
            1,
            2,
            vec![seg(1, 2, Some(info_with_seq_set(vec![1247260])))],
        );
        let result = segfilter_honorific(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn h_e_l_is_noun_r_honor_empty() {
        // Left is a noun particle, right is honorific: result is empty.
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, Some(info_with_seq_set(vec![2028920])))],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(1, 2, Some(info_with_seq_set(vec![1247260])))],
        );
        let result = segfilter_honorific(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn h_f_l_is_noun_r_mixed() {
        // Left is a noun particle, right mixed: left unchanged, right reduced
        // to the non-honorific segment.
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, Some(info_with_seq_set(vec![2028920])))],
        );
        let r = lite_sl(
            1,
            2,
            vec![
                seg(1, 2, Some(info_with_seq_set(vec![1247260]))),
                seg(1, 2, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let result = segfilter_honorific(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn h_g_l_mixed_r_honor() {
        // Left mixes a noun particle with a non-noun, right is honorific: only
        // the non-noun left × right pair survives.
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![2028920]))),
                seg(0, 1, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(1, 2, Some(info_with_seq_set(vec![1247260])))],
        );
        let result = segfilter_honorific(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![999]);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn h_h_gap_r_mixed() {
        // Left not adjacent to right, right mixed: left unchanged, right
        // reduced to the non-honorific segment.
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![999])))]);
        let r = lite_sl(
            2,
            3,
            vec![
                seg(2, 3, Some(info_with_seq_set(vec![1247260]))),
                seg(2, 3, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let result = segfilter_honorific(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn h_i_gap_r_all_honor_empty() {
        // Left not adjacent to right, whole right is honorific: result is
        // empty.
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![999])))]);
        let r = lite_sl(
            2,
            3,
            vec![seg(2, 3, Some(info_with_seq_set(vec![1247260])))],
        );
        let result = segfilter_honorific(Some(&l), &r);
        assert!(result.is_empty());
    }
}

mod segfilter_mononi {
    use crate::dict::grammar::segfilter::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::path::SegmentList;
    use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::dao::SimpleText;

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

    fn seg(start: usize, end: usize, info: Option<KaniSegmentInfo>) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info,
            top: None,
            text: None,
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    #[test]
    fn m_a_l_nil_r_mononi_pass_through() {
        // No left, right is ものに: right passes through.
        let r = lite_sl(
            0,
            1,
            vec![seg(0, 1, Some(info_with_seq_set(vec![1009980])))],
        );
        let result = segfilter_mononi(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn m_b_l_not_mo_r_mononi() {
        // Left is not も, right is ものに: both pass through.
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![999])))]);
        let r = lite_sl(
            1,
            2,
            vec![seg(1, 2, Some(info_with_seq_set(vec![1009980])))],
        );
        let result = segfilter_mononi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn m_c_l_mo_r_mononi_empty() {
        // Left is も, right is ものに: result is empty.
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, Some(info_with_seq_set(vec![2028940])))],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(1, 2, Some(info_with_seq_set(vec![1009980])))],
        );
        let result = segfilter_mononi(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn m_d_l_mixed_mo_r_mononi() {
        // Left mixes も with a non-も word, right is ものに: only the non-も
        // left × right pair survives.
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![2028940]))),
                seg(0, 1, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(1, 2, Some(info_with_seq_set(vec![1009980])))],
        );
        let result = segfilter_mononi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![999]);
        assert_eq!(result[0].1.segments.len(), 1);
    }
}
