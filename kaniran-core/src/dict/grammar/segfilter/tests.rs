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
        // REPL: (classify #'oddp '(1 2 3 4 5)) => yep=(1 3 5) nope=(2 4)
        let (yep, nope) = classify(|n: &i32| n % 2 != 0, &[1, 2, 3, 4, 5]);
        assert_eq!(yep, vec![1, 3, 5]);
        assert_eq!(nope, vec![2, 4]);
    }

    #[test]
    fn empty_input_yields_empty_outputs() {
        // REPL: (classify #'oddp '()) => yep=NIL nope=NIL
        let (yep, nope) = classify(|n: &i32| n % 2 != 0, &[]);
        assert!(yep.is_empty());
        assert!(nope.is_empty());
    }

    #[test]
    fn all_nope_branch() {
        // REPL: (classify #'oddp '(2 4 6)) => yep=NIL nope=(2 4 6)
        let (yep, nope) = classify(|n: &i32| n % 2 != 0, &[2, 4, 6]);
        assert!(yep.is_empty());
        assert_eq!(nope, vec![2, 4, 6]);
    }

    #[test]
    fn all_yep_branch() {
        // REPL: (classify (constantly t) '(1 2 3)) => yep=(1 2 3) nope=NIL
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
    // Synthetic-filter tests pinning each branch of the macro
    // expansion independent of any specific dictionary lookup. The
    // per-callsite segfilter_*.rs files cover the full pipeline with
    // real fixtures; these tests give the helper a self-contained
    // specification.

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
            segments,
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes from `/tmp/probe_aux_verb.lisp` (this session); each
    // assertion below pins a Lisp result line.

    #[test]
    fn a_l_nil_r_no_match() {
        // A l=NIL r=no-match -> {(L=NIL R=[r unchanged 1 seg seq=999])}
        let r = lite_sl(0, 2, vec![seg(0, 2, info_with_seq_set(vec![999]))]);
        let result = segfilter_aux_verb(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn b_l_nil_r_all_match() {
        // B l=NIL r=all-match -> {} (empty — sat-r is full, con-r is empty)
        let r = lite_sl(0, 2, vec![seg(0, 2, info_with_seq_set(vec![1342560]))]);
        let result = segfilter_aux_verb(None, &r);
        assert!(result.is_empty());
    }

    #[test]
    fn c_l_nil_r_mixed() {
        // C l=NIL r=mixed -> {(L=NIL R=[1-seg seq=999])}
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
        // D l=adj-gap (l.end != r.start), r=mixed -> {(L=l unchanged, R=r-reduced-to-non-aux 1 seg)}
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
        // E l-no-sat (cd missing conj-type=13), r=mixed -> {(L=l unchanged 1 seg, R=r-reduced 1 seg)}
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
        // F l-mixed (conj13 + conj3) r-mixed -> two splits:
        //   1st: (L=mslf(l, sat_l)=1 seg, R=mslf(r, sat_r)=1 seg seq=1342560)
        //   2nd: (L=l unchanged 2 segs, R=mslf(r, con_r)=1 seg seq=999)
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

        // First pair: sat-l × sat-r.
        let (lp0, rp0) = &result[0];
        let lp0_ref = lp0.as_ref().unwrap();
        assert_eq!(lp0_ref.segments.len(), 1);
        assert_eq!(rp0.segments.len(), 1);
        assert_eq!(rp0.segments[0].seq_set, vec![1342560]);

        // Second pair: l unchanged × con-r.
        let (lp1, rp1) = &result[1];
        let lp1_ref = lp1.as_ref().unwrap();
        assert_eq!(lp1_ref.segments.len(), 2);
        assert_eq!(rp1.segments.len(), 1);
        assert_eq!(rp1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn g_l_all_sat_r_all_sat() {
        // G l-all-sat r-all-sat -> {(L=l unchanged, R=r unchanged)} (con-l empty path)
        let l = lite_sl(0, 2, vec![seg(0, 2, info_with_conj(vec![cdata(13)]))]);
        let r = lite_sl(2, 4, vec![seg(2, 4, info_with_seq_set(vec![1342560]))]);
        let result = segfilter_aux_verb(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn i_l_all_sat_r_no_match() {
        // I l-all-sat r-no-match -> {(L=l unchanged, R=r unchanged)} (clause-1 path, sat-r empty)
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
        // J l-mixed r-all-sat -> {(L=mslf(l, sat_l), R=mslf(r, sat_r))}  (con-r empty; no base pair)
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes from `/tmp/probe_415_423.lisp` (this session).

    #[test]
    fn n_a_l_nil_r_all_n_pass_through() {
        // N-A l=NIL r=all-n cnt=1 → pass-through (allow-first)
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
        // N-B l=NIL r=no-match cnt=1 → clause-1
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
        // N-C l=NIL r=mixed cnt=1 → pass-through (allow-first); both segs preserved
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
        // N-D l-not-noun r-n cnt=1; sat-l full, con-l empty → (l, r)
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
        // N-E l-is-noun (simple 2028920=は, in *noun-particles*) r-all-n cnt=0
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
        // N-F l-is-noun r-mixed cnt=1 — base pair (l unchanged, mslf r con-r)
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
        // N-G l-mixed (noun + not-noun) r-all-n cnt=1
        // sat-l = not-noun, con-l = noun; sat-r full, con-r empty
        // → sat-l push only → (mslf l sat-l, mslf r sat-r)
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
        // N-H gap r-mixed cnt=1 — clause-2 with con-r non-empty
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
        // N-I gap r-all-n cnt=0 — clause-2 with con-r empty
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
        // N-J l-compound r-all-n cnt=1
        // compound seq is Multi → filter-in-seq-set-simple returns false → complement true
        // → sat-l full, con-l empty → pass through
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes from `/tmp/probe_415_423.lisp` (this session).

    #[test]
    fn w_a_l_nil_r_karasu_empty() {
        // W-A l=NIL r=karasu cnt=0 — no allow-first, clause-2 (not l)=T, con-r empty → ()
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2087020]))]);
        let result = segfilter_wokarasu(None, &r);
        assert!(result.is_empty());
    }

    #[test]
    fn w_b_l_nil_r_mixed() {
        // W-B l=NIL r=mixed cnt=1 — clause-2 (not l)=T, con-r non-empty
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
        // W-C l=NIL r=no-match cnt=1 — clause-1
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let result = segfilter_wokarasu(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn w_d_l_wo_r_karasu_pass_through() {
        // W-D l-wo r-karasu cnt=1 — sat-l full, con-l empty → (l, r)
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2029010]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![2087020]))]);
        let result = segfilter_wokarasu(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn w_e_l_not_wo_r_karasu_empty() {
        // W-E l-not-wo r-karasu cnt=0 — sat-l empty, con-l full; con-r empty → ()
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![2087020]))]);
        let result = segfilter_wokarasu(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn w_f_l_mixed_r_mixed_two_pairs() {
        // W-F l-mixed (wo + other) r-mixed (karasu + other) cnt=2
        //   [0] sat-l × sat-r: L=(wo, 1 seg) × R=(karasu, 1 seg)
        //   [1] l unchanged × con-r: L=(2 segs) × R=(other, 1 seg seq=888)
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

        // First pair: sat-l × sat-r.
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(
            result[0].0.as_ref().unwrap().segments[0].seq_set,
            vec![2029010]
        );
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![2087020]);

        // Second pair: l unchanged × con-r.
        assert_eq!(result[1].0.as_ref().unwrap().segments.len(), 2);
        assert_eq!(result[1].1.segments.len(), 1);
        assert_eq!(result[1].1.segments[0].seq_set, vec![888]);
    }

    #[test]
    fn w_g_gap_r_mixed() {
        // W-G gap (l.end=1, r.start=2) r-mixed cnt=1 — clause-2 with con-r non-empty
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
        // W-H gap r-all-karasu cnt=0 — clause-2 with con-r empty → ()
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes from `/tmp/probe_badend.lisp` (this session).

    #[test]
    fn ba_a_l_nil_r_all_match_returns_empty() {
        // Ba-A l=NIL r=all-match -> {} (allow-first=nil, con-r empty)
        let seg_chai = seg(1, 2, compound(&["ちゃい"]));
        let r = lite_sl(1, 2, vec![seg_chai]);
        let result = segfilter_badend(None, &r);
        assert!(result.is_empty());
    }

    #[test]
    fn ba_b_l_nil_r_mixed() {
        // Ba-B l=NIL r=mixed -> {(L=NIL R=[1 seg = non-matching])}
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
        // Ba-C l=NIL r=no-match -> {(L=NIL R=r unchanged)} (clause-1)
        let seg_x = seg(1, 2, compound(&["x"]));
        let r = lite_sl(1, 2, vec![seg_x]);
        let result = segfilter_badend(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn ba_d_l_adj_r_mixed_emits_base_pair_only() {
        // Ba-D l-adj r=mixed -> {(L=l unchanged 1 seg, R=mslf(r, con_r)=1 seg)}
        // sat-l is always empty (constantly nil) so no prepend.
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
        // Ba-E l-adj r=all-match -> {} (sat-l empty + con-r empty → both branches contribute nothing)
        let seg_simp = seg(0, 1, KaniWordDispatchEnum::Kana(kana("い", 9995)));
        let seg_chai = seg(1, 3, compound(&["ちゃい"]));
        let l = lite_sl(0, 1, vec![seg_simp]);
        let r = lite_sl(1, 3, vec![seg_chai]);
        let result = segfilter_badend(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn ba_f_l_adj_gap_r_mixed() {
        // Ba-F l-adj-gap (l.end=1, r.start=2) r=mixed ->
        //   {(L=l unchanged, R=mslf(r, con_r)=1 seg)}
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
        // Ba-G l-adj r=no-match -> {(L=l unchanged, R=r unchanged)} (clause-1)
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes from `/tmp/probe_415_423.lisp` (this session).

    #[test]
    fn sk_a_l_nil_r_suki_conj54_empty() {
        // SK-A l=NIL r=suki-conj54 cnt=0
        // no allow-first; sat-r full, l=NIL → clause-2 (not l)=T, con-r empty → ()
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
        // SK-B l=NIL r=mixed cnt=1
        // clause-2 (not l)=T, con-r non-empty → (NIL, mslf r con-r)
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
        // SK-C l=NIL r=no-match cnt=1 — clause-1
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
        // SK-D l-simple r-suki cnt=0
        // sat-l empty (constantly nil), con-l full; sat-r full, con-r empty
        // → no base, no sat-l push (sat-l empty) → empty
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
        // SK-E l-simple r-mixed cnt=1
        // sat-l empty, con-l full; sat-r=suki, con-r=other → base pair only
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
        // SK-F gap r-suki cnt=0 — clause-2 (l.end != r.start) with con-r empty → ()
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
        // SK-G l=NIL r=suki-no-conj cnt=1 — filter-right requires conj-54; without it sat-r empty
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
        // SK-H l=NIL r=conj54-not-suki cnt=1 — text doesn't end with 好き → sat-r empty
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes from `/tmp/probe_415_423.lisp` (this session).

    #[test]
    fn r_a_l_nil_r_ku_pass_through() {
        // R-A l=NIL r=ku cnt=1 — allow-first pass-through
        let r = lite_sl(0, 1, vec![simple_seg(0, 1, "くる", 100)]);
        let result = segfilter_roku(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn r_b_l_nil_r_not_ku() {
        // R-B l=NIL r=not-ku cnt=1 — clause-1
        let r = lite_sl(0, 1, vec![simple_seg(0, 1, "あさ", 100)]);
        let result = segfilter_roku(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn r_c_l_simple_r_ku_pass_through() {
        // R-C l-simple r-ku cnt=1 — sat-l full, con-l empty → (l, r)
        let l = lite_sl(0, 1, vec![simple_seg(0, 1, "abc", 999)]);
        let r = lite_sl(1, 2, vec![simple_seg(1, 2, "くる", 100)]);
        let result = segfilter_roku(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn r_d_l_iro_r_ku_empty() {
        // R-D l-iro r-ku cnt=0 — sat-l empty, con-l full; con-r empty → ()
        let l = lite_sl(0, 2, vec![compound_ending_seg(0, 2, "いろ", 50)]);
        let r = lite_sl(2, 3, vec![simple_seg(2, 3, "くる", 100)]);
        let result = segfilter_roku(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn r_e_l_mixed_iro_r_ku_sat_push() {
        // R-E l-mixed (compound-iro + simple) r-ku cnt=1
        // sat-l=simple, con-l=compound-iro; con-r empty → sat-l push only
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
        // R-F gap r-mixed cnt=1 — clause-2 with con-r non-empty (only the non-ku reading survives)
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes from `/tmp/probe_415_423.lisp` (this session).

    #[test]
    fn s_a_l_nil_r_e_pass_through() {
        // S-A l=NIL r=e cnt=1 — allow-first pass-through
        let r = lite_sl(0, 1, vec![simple_seg(0, 1, "える", 100, None)]);
        let result = segfilter_sae(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn s_b_l_nil_r_not_e() {
        // S-B l=NIL r=not-e cnt=1 — clause-1
        let r = lite_sl(0, 1, vec![simple_seg(0, 1, "abc", 100, None)]);
        let result = segfilter_sae(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn s_c_l_simple_r_e() {
        // S-C l-simple r-e cnt=1 — sat-l full, con-l empty → (l, r)
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
        // S-D l-comp-end-2029120 r-e cnt=0
        let l = lite_sl(0, 2, vec![compound_ending_seg(0, 2, 2029120)]);
        let r = lite_sl(2, 3, vec![simple_seg(2, 3, "える", 100, None)]);
        let result = segfilter_sae(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn s_e_l_mixed_r_e_sat_push() {
        // S-E l-mixed (compound-end-sae + simple) r-e cnt=1
        // sat-l=simple, con-l=compound; con-r empty → sat-l push only
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes from `/tmp/probe_410_414.lisp` (this session).

    #[test]
    fn j_a_l_nil_r_janai_pass_through() {
        // J-A l=NIL r=janai cnt=1 r-segs=1
        // allow-first → (list (list nil r))
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
        // J-B l-simple r-janai cnt=1 l-segs=1
        // simple-l → (filter-is-compound-end 2028920) returns NIL → complement → T
        // sat-l full, con-l empty → (list (list l r))
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
        // J-C l-compound-end-wa r-janai => NIL
        // compound ending in 2028920 → filter-is-compound-end T → complement NIL
        // sat-l empty, con-l full; sat-r full, con-r empty → empty result
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
        // J-D l-mixed-comp r-janai cnt=1 l-segs=1 l0-info=(999)
        // sat-l = simple (not compound-end-ha), con-l = compound-end-ha
        // sat-r full, con-r empty → base skipped; sat-l push → 1 pair
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
        // J-E gap janai-mixed cnt=1 r-segs=1 r-info=(999)
        // clause-2 (l.end=1 != r.start=2) with con-r non-empty
        // → (list (list l (mslf r con-r)))
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes from `/tmp/probe_415_423.lisp` (this session).

    #[test]
    fn nh_a_l_nil_r_match() {
        // NH-A l=NIL r=match cnt=1 — pass-through (allow-first)
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![1601080]))]);
        let result = segfilter_nohayamete(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn nh_b_l_nil_r_no_match() {
        // NH-B l=NIL r=no-match cnt=1 — clause-1
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let result = segfilter_nohayamete(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn nh_c_l_not_no_r_hayamete() {
        // NH-C l-not-no r-hayamete cnt=1
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1601080]))]);
        let result = segfilter_nohayamete(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn nh_d_l_is_no_r_hayamete_empty() {
        // NH-D l-is-no r-hayamete cnt=0
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![1469800]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1601080]))]);
        let result = segfilter_nohayamete(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn nh_e_l_mixed_r_hayamete() {
        // NH-E l-mixed r-hayamete cnt=1 — sat-l push (con-r empty)
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
        // NH-F gap r-mixed cnt=1 — clause-2 with con-r non-empty
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes from `/tmp/probe_415_423.lisp` (this session).

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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes from `/tmp/probe_dashi.lisp` (this session).

    #[test]
    fn da_a_l_nil_r_all_match_passes_through_allow_first() {
        // Da-A l=NIL r-all-match -> {(L=NIL R=[1-seg seq=1157170])} (allow-first short-circuit)
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
        // Da-B l=NIL r-mixed -> {(L=NIL R=[2-segs unchanged])}
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
        // Da-C l-da r-no-match -> {(L=l unchanged 1 seg, R=r unchanged 1 seg seq=999)}
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
        // Da-D l-sat-l (no 2089020) r-sat-r (suru) -> {(L=l unchanged, R=r unchanged)}
        // (sat-l = all of l; con-l empty; falls through to "(list (list l r))")
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
        // Da-E l-da r-mixed -> {(L=l unchanged 1 seg, R=mslf(r, con_r)=1 seg seq=999)}
        // l has only da: sat-l empty (da fails left filter), con-l full.
        // Only the base pair emits (sat-l prepend skipped).
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
        // Da-F l-de (sat-l: has 2028980) r-suru -> {(L=l unchanged, R=r unchanged)} (con-l empty)
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
        // Da-G l-da-then-da+de (sat-l = 2nd seg has で; con-l = 1st seg da-only) r-suru ->
        // {(L=mslf(l, sat_l)=1 seg [da+de], R=mslf(r, sat_r)=1 seg)} (no base pair: con_r empty)
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
        // Da-H l-da (l.end=1) r-mixed-gap (r.start=2) -> {(L=l unchanged, R=mslf(r, con_r)=1 seg)}
        // l.end != r.start; con-r non-empty.
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
        // Da-I l-info-nil r-suru -> {(L=l unchanged, R=r unchanged)}
        // info=None ⇒ seq-set empty ⇒ left filter truthy (not (find da empty)=t).
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes from `/tmp/probe_dekiru.lisp` (this session).

    #[test]
    fn de_a_l_nil_r_all_match() {
        // De-A l=NIL r-all-match -> {(L=NIL R=[1 seg seq=2830009])} (allow-first)
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
        // De-B l=NIL r-mixed -> {(L=NIL R=[r unchanged, 2 segs])}
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
        // De-C l-de r-no-match -> {(L=l unchanged, R=r unchanged)} (sat-r empty)
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
        // De-D l-sat-l (not 出) r-sat-r (来る) -> {(L=l unchanged, R=r unchanged)} (con-l empty)
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
        // De-E l-de r-mixed -> {(L=l unchanged 1 seg, R=mslf(r, con_r)=1 seg seq=999)}
        // l = only 出 → sat-l empty (complement of in-de-seqs is false), con-l = l's seg.
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
        // De-F l-mixed (1896380=出 fails, 888 sat) r-mixed -> two splits:
        //   1st: (L=mslf(l, sat_l)=1 seg [888], R=mslf(r, sat_r)=1 seg [2830009])
        //   2nd: (L=l unchanged 2 segs, R=mslf(r, con_r)=1 seg [999])
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

        // First pair: sat × sat.
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![888]);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![2830009]);

        // Second pair: l unchanged × con_r.
        assert_eq!(result[1].0.as_ref().unwrap().segments.len(), 2);
        assert_eq!(result[1].1.segments.len(), 1);
        assert_eq!(result[1].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn de_g_l_mixed_r_all_sat() {
        // De-G l-mixed r-all-sat -> {(L=mslf(l, sat_l)=1 seg, R=mslf(r, sat_r)=1 seg)} (con_r empty)
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
        // De-H l-info-nil r-sat -> {(L=l unchanged, R=r unchanged)}
        // info=None ⇒ seq-set empty ⇒ inner filter returns false ⇒ complement returns true ⇒
        // sat-l = full, con-l empty, falls through to "(list (list l r))".
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes from `/tmp/probe_410_414.lisp` (this session).

    #[test]
    fn h_a_l_nil_r_all_honor_empty() {
        // H-A l=NIL r=all-honorific => NIL
        // sat-r full, allow-first=nil, clause-2 (not l)=t, con-r empty → ()
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
        // H-B l=NIL r=mixed cnt=1
        // clause-2 with (not l)=t, con-r non-empty → (list (list nil (mslf r con-r)))
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
        // H-C l=NIL r=no-match cnt=1 l0=NIL r0-segs=1
        // sat-r empty → clause-1 → (list (list l r))
        let r = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![999])))]);
        let result = segfilter_honorific(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn h_d_l_not_noun_r_honor() {
        // H-D l-not-noun r-honor cnt=1 l-segs=1 r-segs=1
        // sat-l = full (not in noun particles), con-l empty → (list (list l r))
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
        // H-E l-is-noun r-honor cnt=0
        // sat-l empty, con-l full, sat-r full, con-r empty → empty result
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
        // H-F l-is-noun r-mixed cnt=1 r0-segs=1 r0-seq=(999)
        // sat-l empty, con-l full, sat-r=honor, con-r=other
        // → base pair (l unchanged, mslf r con-r); no sat-l prepend
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
        // H-G l-mixed r-honor cnt=1 l-segs=1 r-segs=1
        // sat-l=non-noun, con-l=noun, sat-r=full, con-r=empty
        // → base skipped; sat-l prepend → 1 pair (mslf l sat-l, mslf r sat-r)
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
        // H-H gap mixed cnt=1 r-segs=1
        // clause-2 (l.end != r.start) with con-r non-empty
        // → (list (list l (mslf r con-r)))
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
        // H-I gap all-honor => NIL
        // clause-2 gap with con-r empty → ()
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes from `/tmp/probe_410_414.lisp` (this session).

    #[test]
    fn m_a_l_nil_r_mononi_pass_through() {
        // M-A l=NIL r=mononi cnt=1
        // allow-first → clause-1 → (list (list nil r))
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
        // M-B l-not-mo r-mononi cnt=1 l-segs=1
        // sat-l full (not mo), con-l empty → (list (list l r))
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
        // M-C l-mo r-mononi => NIL
        // sat-l empty, con-l full, sat-r full, con-r empty → empty result
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
        // M-D l-mixed-mo r-mononi cnt=1 l-segs=1 l0-info=(999)
        // sat-l = non-mo, con-l = mo, sat-r full, con-r empty
        // → base skipped; sat-l push → 1 pair (mslf l sat-l, mslf r sat-r)
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
