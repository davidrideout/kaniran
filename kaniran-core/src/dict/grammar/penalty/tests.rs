mod synergy_no_toori {
    use crate::dict::conj::ConjData;
    use crate::dict::grammar::penalty::*;
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
    fn positive_no_toori() {
        // Right span and left span feed through, producing the "no toori"
        // synergy with score 50.
        let l = lite_sl_owned(0, 1, vec![seg_with_seqs(vec![1469800])]);
        let r = lite_sl_owned(1, 3, vec![seg_with_seqs(vec![1432920])]);
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
        let l = lite_sl_owned(0, 1, vec![seg_with_seqs(vec![12345])]);
        let r = lite_sl_owned(1, 3, vec![seg_with_seqs(vec![1432920])]);
        assert!(synergy_no_toori(&l, &r).is_empty());
    }

    #[test]
    fn multi_segs_partial_filter() {
        // Left has two segments (one matches, one does not), right has
        // two (both match): right keeps both, left keeps the one match.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg_with_seqs(vec![1469800]), seg_with_seqs(vec![99])],
        );
        let r = lite_sl_owned(
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

mod synergy_oki {
    use crate::dict::conj::ConjData;
    use crate::dict::grammar::penalty::*;
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

    fn seg(kpcl: (bool, bool, bool, bool), posi: Vec<&str>, seq_set: Vec<i32>) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
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
            }),
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
    fn positive_2854117() {
        // Counter on the left followed by seq 2854117 fires the synergy:
        // no description, score 20.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, false), vec!["ctr"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            3,
            vec![seg((false, false, false, false), vec![], vec![2854117])],
        );
        let got = synergy_oki(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert!(syn.description.is_none());
        assert_eq!(syn.connector.as_deref(), Some(""));
        assert_eq!(syn.score, 20);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_2084550() {
        // The other accepted right sequence also fires.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, false), vec!["ctr"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            3,
            vec![seg((false, false, false, false), vec![], vec![2084550])],
        );
        let got = synergy_oki(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 20);
    }

    #[test]
    fn neg_no_ctr_posi() {
        // Left is a noun, not a counter — no synergy.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            3,
            vec![seg((false, false, false, false), vec![], vec![2854117])],
        );
        assert!(synergy_oki(&l, &r).is_empty());
    }

    #[test]
    fn neg_right_miss() {
        // Right sequence matches neither accepted value — no synergy.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((true, false, false, false), vec!["ctr"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            3,
            vec![seg((false, false, false, false), vec![], vec![9999999])],
        );
        assert!(synergy_oki(&l, &r).is_empty());
    }

    #[test]
    fn neg_not_adjacent() {
        // Left and right are not adjacent — no synergy.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, false), vec!["ctr"], vec![])],
        );
        let r = lite_sl_owned(
            5,
            7,
            vec![seg((false, false, false, false), vec![], vec![2854117])],
        );
        assert!(synergy_oki(&l, &r).is_empty());
    }
}

mod get_synergies {
    use crate::dict::conj::ConjData;
    use crate::dict::grammar::penalty::*;
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

    fn seg(kpcl: (bool, bool, bool, bool), posi: Vec<&str>, seq_set: Vec<i32>) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
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
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    fn unwrap_synergy(path: &[KaniLitePathElement]) -> &crate::dict::grammar::synergy::Synergy {
        match &path[1] {
            KaniLitePathElement::Synergy(s) => s,
            other => panic!("expected Synergy at [1], got {:?}", other),
        }
    }

    fn unwrap_sl(elem: &KaniLitePathElement) -> &KaniLiteSegmentList {
        match elem {
            KaniLitePathElement::SegmentList(sl) => sl,
            other => panic!("expected SegmentList, got {:?}", other),
        }
    }

    #[test]
    fn a_none_fire() {
        let l = lite_sl(
            0,
            1,
            vec![seg((true, false, false, false), vec!["zzz"], vec![9999])],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg((false, false, false, false), vec!["zzz"], vec![8888])],
        );
        assert!(get_synergies(&l, &r).is_empty());
    }

    #[test]
    fn b_only_no_adjectives() {
        let l = lite_sl(
            0,
            1,
            vec![seg((true, false, false, false), vec!["adj-no"], vec![])],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![1469800])],
        );
        let got = get_synergies(&l, &r);
        assert_eq!(got.len(), 1);
        let syn = unwrap_synergy(&got[0]);
        assert_eq!(syn.description.as_deref(), Some("no-adjective"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(unwrap_sl(&got[0][0]).start, 1);
        assert_eq!(unwrap_sl(&got[0][0]).end, 2);
        assert_eq!(unwrap_sl(&got[0][2]).start, 0);
        assert_eq!(unwrap_sl(&got[0][2]).end, 1);
    }

    #[test]
    fn c_only_to_adverbs() {
        let l = lite_sl(
            0,
            2,
            vec![seg((true, false, false, false), vec!["adv-to"], vec![])],
        );
        let r = lite_sl(
            2,
            3,
            vec![seg((false, false, false, false), vec![], vec![1008490])],
        );
        let got = get_synergies(&l, &r);
        assert_eq!(got.len(), 1);
        let syn = unwrap_synergy(&got[0]);
        assert_eq!(syn.description.as_deref(), Some("to-adverb"));
        assert_eq!(syn.score, 30);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
    }

    #[test]
    fn d_noun_da_only() {
        let l = lite_sl(
            0,
            1,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![2089020])],
        );
        let got = get_synergies(&l, &r);
        assert_eq!(got.len(), 1);
        let syn = unwrap_synergy(&got[0]);
        assert_eq!(syn.description.as_deref(), Some("noun+da"));
        assert_eq!(syn.score, 10);
    }

    #[test]
    fn e_noun_particle_only() {
        let l = lite_sl(
            0,
            1,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![2028920])],
        );
        let got = get_synergies(&l, &r);
        assert_eq!(got.len(), 1);
        let syn = unwrap_synergy(&got[0]);
        assert_eq!(syn.description.as_deref(), Some("noun+prt"));
        assert_eq!(syn.score, 14);
    }

    #[test]
    fn f_two_synergies_order_mirrors_synergy_list() {
        let l = lite_sl(
            0,
            1,
            vec![seg(
                (true, false, false, false),
                vec!["adj-no"],
                vec![1469800],
            )],
        );
        let r = lite_sl(
            1,
            2,
            vec![
                seg((false, false, false, false), vec![], vec![1469800]),
                seg((false, false, false, false), vec![], vec![2089020]),
            ],
        );
        let got = get_synergies(&l, &r);
        assert_eq!(got.len(), 2);
        let syn0 = unwrap_synergy(&got[0]);
        assert_eq!(syn0.description.as_deref(), Some("no-adjective"));
        assert_eq!(syn0.score, 15);
        let syn1 = unwrap_synergy(&got[1]);
        assert_eq!(syn1.description.as_deref(), Some("no da/desu"));
        assert_eq!(syn1.score, 15);
    }

    #[test]
    fn g_non_adjacent() {
        let l = lite_sl(
            0,
            1,
            vec![seg((true, false, false, false), vec!["adj-no"], vec![])],
        );
        let r = lite_sl(
            5,
            6,
            vec![seg((false, false, false, false), vec![], vec![1469800])],
        );
        assert!(get_synergies(&l, &r).is_empty());
    }
}

mod filter_short_kana {
    use crate::dict::conj::ConjData;
    use crate::dict::grammar::penalty::*;
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

    fn seg(start: usize, end: usize, info: Option<KaniSegmentInfo>, text: Option<&str>) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info,
            top: None,
            text: text.map(str::to_string),
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments: segments.into_iter().map(std::sync::Arc::new).collect(),
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    #[test]
    fn c1_empty_segments_is_false() {
        let f = filter_short_kana(1, vec![]);
        assert!(!f(&lite_sl(0, 1, vec![])));
    }

    #[test]
    fn c2_span_exceeds_len_is_false() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(
            0,
            2,
            Some(info_with((false, false, false, false), vec![999])),
            Some("あい"),
        );
        assert!(!f(&lite_sl(0, 2, vec![s])));
    }

    #[test]
    fn c3_kpcl_first_set_is_false() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(
            0,
            1,
            Some(info_with((true, false, false, false), vec![999])),
            Some("あ"),
        );
        assert!(!f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c4_all_pass_no_except_is_true() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(
            0,
            1,
            Some(info_with((false, false, false, false), vec![999])),
            Some("あ"),
        );
        assert!(f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c5_except_matches_text_is_false() {
        let f = filter_short_kana(1, vec!["と".to_string()]);
        let s = seg(
            0,
            1,
            Some(info_with((false, false, false, false), vec![999])),
            Some("と"),
        );
        assert!(!f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c6_except_differs_from_text_is_true() {
        let f = filter_short_kana(1, vec!["と".to_string()]);
        let s = seg(
            0,
            1,
            Some(info_with((false, false, false, false), vec![999])),
            Some("あ"),
        );
        assert!(f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c8_kpcl_second_set_first_nil_is_true() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(
            0,
            1,
            Some(info_with((false, true, false, false), vec![999])),
            Some("あ"),
        );
        assert!(f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c9_no_info_plist_is_true() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(0, 1, None, Some("あ"));
        assert!(f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c10_span_equals_len_is_true() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(
            5,
            6,
            Some(info_with((false, false, false, false), vec![999])),
            Some("あ"),
        );
        assert!(f(&lite_sl(5, 6, vec![s])));
    }

    #[test]
    fn c11_only_first_seg_examined() {
        let f = filter_short_kana(1, vec![]);
        let s_good = seg(
            0,
            1,
            Some(info_with((false, false, false, false), vec![999])),
            Some("あ"),
        );
        let s_kpcl = seg(
            0,
            1,
            Some(info_with((true, false, false, false), vec![888])),
            Some("あ"),
        );
        assert!(f(&lite_sl(0, 1, vec![s_good, s_kpcl])));
    }

    #[test]
    fn c12_no_except_kw_text_to_is_true() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(
            0,
            1,
            Some(info_with((false, false, false, false), vec![999])),
            Some("と"),
        );
        assert!(f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c13_except_empty_text_to_is_true() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(
            0,
            1,
            Some(info_with((false, false, false, false), vec![999])),
            Some("と"),
        );
        assert!(f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c14_len_two_span_two_is_true() {
        let f = filter_short_kana(2, vec![]);
        let s = seg(
            0,
            2,
            Some(info_with((false, false, false, false), vec![999])),
            Some("あい"),
        );
        assert!(f(&lite_sl(0, 2, vec![s])));
    }
}
