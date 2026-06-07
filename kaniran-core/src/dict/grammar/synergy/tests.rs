mod make_segment_list_from {
    use crate::dict::grammar::synergy::*;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
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

    fn seg_with_score(score: i32) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: Some(score),
            info: None,
            top: None,
            text: None,
        }
    }

    #[test]
    fn swaps_segments_preserves_other_slots() {
        // REPL:
        //   src segments len=2, dst segments len=1
        //   dst start=0 end=2 matches=3
        //   src not mutated: src segments len=2
        //   first dst seg score=20
        let seg1 = seg_with_score(10);
        let seg2 = seg_with_score(20);
        let sl = SegmentList {
            segments: vec![seg1.clone(), seg2.clone()],
            start: 0,
            end: 2,
            top: None,
            matches: 3,
        };
        let new_sl = make_segment_list_from(&sl, vec![seg2.clone()]);
        assert_eq!(sl.segments.len(), 2);
        assert_eq!(new_sl.segments.len(), 1);
        assert_eq!(new_sl.start, 0);
        assert_eq!(new_sl.end, 2);
        assert_eq!(new_sl.matches, 3);
        assert_eq!(new_sl.segments[0].score, Some(20));
    }
}

mod filter_is_pos_macro {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_lite_segment::{POS_ADJ_NA, POS_ADJ_NO, POS_ADV_TO, POS_CTR, POS_N};
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

    fn lite(kpcl: (bool, bool, bool, bool), posi: &[&str]) -> Arc<KaniLiteSegment> {
        let info = KaniSegmentInfo {
            posi: posi.iter().map(|s| s.to_string()).collect(),
            seq_set: vec![],
            conj: vec![] as Vec<ConjData>,
            common: None,
            score_info: KaniScoreInfo {
                prop_score: 0,
                kanji_break: vec![],
                use_length_bonus: 0,
                split_info: KaniSplitInfo::None,
            },
            kpcl,
        };
        Arc::new(KaniLiteSegment::from_segment(Arc::new(Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(info),
            top: None,
            text: None,
        })))
    }

    // kpcl-test bodies used at the upstream `filter-is-pos` callsites
    // plus a few that isolate the kpcl gate from the pos gate.
    fn adj(k: bool, p: bool, c: bool, l: bool) -> bool {
        k || l || (p && c)
    } // dict-grammar.lisp:864/871 (or k l (and p c))
    fn advto(k: bool, p: bool, _c: bool, l: bool) -> bool {
        k || l || p
    } // dict-grammar.lisp:878 (or k l p)
    fn orkl(k: bool, _p: bool, _c: bool, l: bool) -> bool {
        k || l
    } // dict-grammar.lisp:915 (or k l)
    fn konly(k: bool, _p: bool, _c: bool, _l: bool) -> bool {
        k
    } // dict-grammar.lisp:922 (k)
    fn always(_k: bool, _p: bool, _c: bool, _l: bool) -> bool {
        true
    } // dict-grammar.lisp:952 (t)
    fn ponly(_k: bool, p: bool, _c: bool, _l: bool) -> bool {
        p
    } // isolation case
    fn pandc(_k: bool, p: bool, c: bool, _l: bool) -> bool {
        p && c
    } // isolation case

    #[test]
    fn filter_is_pos_fixtures() {
        // REPL fixtures (.103, `ichiran/dict::filter-is-pos` applied to
        // a `gen-score`d segment), 2026-05-24. Columns:
        // (label, kpcl (k p c l), posi, pos_mask, kpcl_test, expected).
        type Test = fn(bool, bool, bool, bool) -> bool;
        let cases: &[(&str, (bool, bool, bool, bool), &[&str], u16, Test, bool)] = &[
            // 普通 — kpcl=(T T T NIL) posi=(adj-na adj-no adv n)
            (
                "futsuu adj-no/adj",
                (true, true, true, false),
                &["adj-na", "adj-no", "adv", "n"],
                POS_ADJ_NO,
                adj,
                true,
            ),
            (
                "futsuu adj-na/adj",
                (true, true, true, false),
                &["adj-na", "adj-no", "adv", "n"],
                POS_ADJ_NA,
                adj,
                true,
            ),
            (
                "futsuu adv-to/advto",
                (true, true, true, false),
                &["adj-na", "adj-no", "adv", "n"],
                POS_ADV_TO,
                advto,
                false,
            ),
            (
                "futsuu n/orkl",
                (true, true, true, false),
                &["adj-na", "adj-no", "adv", "n"],
                POS_N,
                orkl,
                true,
            ),
            (
                "futsuu ctr/t",
                (true, true, true, false),
                &["adj-na", "adj-no", "adv", "n"],
                POS_CTR,
                always,
                false,
            ),
            // 政府 — kpcl=(T T T NIL) posi=(n)
            (
                "seifu adj-no/adj",
                (true, true, true, false),
                &["n"],
                POS_ADJ_NO,
                adj,
                false,
            ),
            (
                "seifu n/orkl",
                (true, true, true, false),
                &["n"],
                POS_N,
                orkl,
                true,
            ),
            // 静か — kpcl=(T T T NIL) posi=(adj-na)
            (
                "shizuka adj-na/adj",
                (true, true, true, false),
                &["adj-na"],
                POS_ADJ_NA,
                adj,
                true,
            ),
            (
                "shizuka n/orkl",
                (true, true, true, false),
                &["adj-na"],
                POS_N,
                orkl,
                false,
            ),
            // 個 — kpcl=(T T T NIL) posi=(ctr n)
            (
                "ko ctr/t",
                (true, true, true, false),
                &["ctr", "n"],
                POS_CTR,
                always,
                true,
            ),
            // 三 — kpcl=(T T T NIL) posi=(num): num maps to no bit → empty intersection
            (
                "san adj-no/adj (num→0)",
                (true, true, true, false),
                &["num"],
                POS_ADJ_NO,
                adj,
                false,
            ),
            (
                "san n/orkl (num→0)",
                (true, true, true, false),
                &["num"],
                POS_N,
                orkl,
                false,
            ),
            // ゆっくり — kpcl=(NIL T T NIL) posi=(adv adv-to vs)
            (
                "yukkuri adv-to/advto (k=F)",
                (false, true, true, false),
                &["adv", "adv-to", "vs"],
                POS_ADV_TO,
                advto,
                true,
            ),
            (
                "yukkuri adv-to/konly (pos-match,test-F)",
                (false, true, true, false),
                &["adv", "adv-to", "vs"],
                POS_ADV_TO,
                konly,
                false,
            ),
            (
                "yukkuri adj-no/adj",
                (false, true, true, false),
                &["adv", "adv-to", "vs"],
                POS_ADJ_NO,
                adj,
                false,
            ),
            // 本 — kpcl=(T NIL T NIL) posi=(ctr n)
            (
                "hon ctr/t",
                (true, false, true, false),
                &["ctr", "n"],
                POS_CTR,
                always,
                true,
            ),
            (
                "hon n/konly (k=T)",
                (true, false, true, false),
                &["ctr", "n"],
                POS_N,
                konly,
                true,
            ),
            (
                "hon n/ponly (pos-match,test-F)",
                (true, false, true, false),
                &["ctr", "n"],
                POS_N,
                ponly,
                false,
            ),
            (
                "hon n/pandc (pos-match,test-F)",
                (true, false, true, false),
                &["ctr", "n"],
                POS_N,
                pandc,
                false,
            ),
            (
                "hon adj-no/adj (p=F)",
                (true, false, true, false),
                &["ctr", "n"],
                POS_ADJ_NO,
                adj,
                false,
            ),
        ];
        for (label, kpcl, posi, pos_mask, kpcl_test, expected) in cases {
            let seg = lite(*kpcl, posi);
            let predicate = filter_is_pos(*pos_mask, kpcl_test);
            assert_eq!(predicate(&seg), *expected, "case={label}");
        }
    }
}

mod filter_in_seq_set {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
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

    fn lite_with_seq_set(seq_set: Vec<i32>) -> Arc<KaniLiteSegment> {
        let info = KaniSegmentInfo {
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
        };
        Arc::new(KaniLiteSegment::from_segment(Arc::new(Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(info),
            top: None,
            text: None,
        })))
    }

    fn lite_no_info() -> Arc<KaniLiteSegment> {
        Arc::new(KaniLiteSegment::from_segment(Arc::new(Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: None,
            top: None,
            text: None,
        })))
    }

    #[test]
    fn match_when_intersection_nonempty() {
        // REPL: filter (200 400) on seg-a (:seq-set (100 200)) -> truthy=T
        let seg = lite_with_seq_set(vec![100, 200]);
        let f = filter_in_seq_set(vec![200, 400]);
        assert!(f(&seg));
    }

    #[test]
    fn no_match_when_disjoint() {
        // REPL: filter (200 400) on seg-b (:seq-set (300)) -> truthy=NIL
        let seg = lite_with_seq_set(vec![300]);
        let f = filter_in_seq_set(vec![200, 400]);
        assert!(!f(&seg));
    }

    #[test]
    fn no_match_when_info_absent() {
        // REPL: filter (200 400) on seg-no-info -> truthy=NIL
        let seg = lite_no_info();
        let f = filter_in_seq_set(vec![200, 400]);
        assert!(!f(&seg));
    }

    #[test]
    fn empty_seqs_never_matches() {
        // REPL: (filter-in-seq-set) on seg-a -> truthy=NIL
        let seg = lite_with_seq_set(vec![100, 200]);
        let f = filter_in_seq_set(vec![]);
        assert!(!f(&seg));
    }
}

mod synergy_noun_particle {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_437_441.lisp on .103, 2026-05-18).

    #[test]
    fn positive_len1() {
        // noun-particle/positive-len1: l noun, r seq 2028920 (は), r.end-r.start=1.
        // SYNERGY desc="noun+prt" conn=" " score=14 start=1 end=1.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![2028920])],
        );
        let got = synergy_noun_particle(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 2);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("noun+prt"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 14);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_len2() {
        // noun-particle/positive-len2: r seq 2215430 (には), span=2 -> score=18.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            3,
            vec![seg((false, false, false, false), vec![], vec![2215430])],
        );
        let got = synergy_noun_particle(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 18);
        assert_eq!(got[0].1.start, 1);
        assert_eq!(got[0].1.end, 1);
    }

    #[test]
    fn positive_len4() {
        // noun-particle/positive-len4: r seq 1009600 (にとって), span=4 -> score=26.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            5,
            vec![seg((false, false, false, false), vec![], vec![1009600])],
        );
        let got = synergy_noun_particle(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 26);
    }

    #[test]
    fn not_adjacent_empty() {
        // noun-particle/not-adjacent: NIL.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            5,
            6,
            vec![seg((false, false, false, false), vec![], vec![2028920])],
        );
        assert!(synergy_noun_particle(&l, &r).is_empty());
    }

    #[test]
    fn right_misses_empty() {
        // noun-particle/right-misses: NIL.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![9999999])],
        );
        assert!(synergy_noun_particle(&l, &r).is_empty());
    }
}

mod synergy_noun_da {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_437_441.lisp on .103, 2026-05-18).

    #[test]
    fn positive() {
        // noun-da/positive: l noun (kpcl k=T posi=("n")), r seq 2089020 (だ).
        // RIGHT-SL start=1 end=2 segs=1, SYNERGY desc="noun+da"
        // conn=" " score=10 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![2089020])],
        );
        let got = synergy_noun_da(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 2);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("noun+da"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 10);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn not_adjacent_empty() {
        // noun-da/not-adjacent: NIL.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            5,
            6,
            vec![seg((false, false, false, false), vec![], vec![2089020])],
        );
        assert!(synergy_noun_da(&l, &r).is_empty());
    }

    #[test]
    fn left_not_noun_empty() {
        // noun-da/left-not-noun: NIL.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((true, false, false, false), vec!["v5k"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![2089020])],
        );
        assert!(synergy_noun_da(&l, &r).is_empty());
    }

    #[test]
    fn right_misses_empty() {
        // noun-da/right-misses: NIL.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![9999999])],
        );
        assert!(synergy_noun_da(&l, &r).is_empty());
    }
}

mod synergy_no_da {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_synergies.lisp on .103, 2026-05-18).

    #[test]
    fn positive_1469800_2089020() {
        // no-da/positive-1: l ends at 2, r starts at 2.
        // RIGHT-SL start=2 end=3 segs=1, SYNERGY desc="no da/desu"
        // conn=" " score=15 start=2 end=2, LEFT-SL start=0 end=2 segs=1
        let l = lite_sl_owned(0, 2, vec![seg_with_seqs(vec![1469800, 999])]);
        let r = lite_sl_owned(2, 3, vec![seg_with_seqs(vec![2089020])]);
        let got = synergy_no_da(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("no da/desu"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_2139720_1928670() {
        // no-da/positive-2.
        let l = lite_sl_owned(0, 1, vec![seg_with_seqs(vec![2139720])]);
        let r = lite_sl_owned(1, 2, vec![seg_with_seqs(vec![1928670])]);
        let got = synergy_no_da(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 15);
        assert_eq!(got[0].1.start, 1);
        assert_eq!(got[0].1.end, 1);
    }

    #[test]
    fn not_adjacent_empty() {
        // no-da/not-adjacent: NIL
        let l = lite_sl_owned(0, 1, vec![seg_with_seqs(vec![1469800])]);
        let r = lite_sl_owned(5, 6, vec![seg_with_seqs(vec![2089020])]);
        assert!(synergy_no_da(&l, &r).is_empty());
    }

    #[test]
    fn left_misses_empty() {
        // no-da/left-misses: NIL
        let l = lite_sl_owned(0, 1, vec![seg_with_seqs(vec![9999999])]);
        let r = lite_sl_owned(1, 2, vec![seg_with_seqs(vec![2089020])]);
        assert!(synergy_no_da(&l, &r).is_empty());
    }

    #[test]
    fn right_misses_empty() {
        // no-da/right-misses: NIL
        let l = lite_sl_owned(0, 1, vec![seg_with_seqs(vec![1469800])]);
        let r = lite_sl_owned(1, 2, vec![seg_with_seqs(vec![9999999])]);
        assert!(synergy_no_da(&l, &r).is_empty());
    }

    #[test]
    fn empty_left_segments() {
        // no-da/empty-left: NIL
        let l = lite_sl_owned(0, 1, vec![]);
        let r = lite_sl_owned(1, 2, vec![seg_with_seqs(vec![2089020])]);
        assert!(synergy_no_da(&l, &r).is_empty());
    }
}

mod synergy_sou_nanda {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
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

    fn seg(start: usize, end: usize, seq_set: Vec<i32>) -> Segment {
        Segment {
            start,
            end,
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_442_448.lisp on .103, 2026-05-18).

    #[test]
    fn positive() {
        // sou-nanda/positive: RIGHT-SL start=2 end=5 segs=1,
        // SYN desc="sou na n da" conn=" " score=50 start=2 end=2,
        // LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![2137720])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![2140410])]);
        let got = synergy_sou_nanda(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 5);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("sou na n da"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 50);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn right_miss_empty() {
        // sou-nanda/right-miss: NIL.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![2137720])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![99])]);
        assert!(synergy_sou_nanda(&l, &r).is_empty());
    }

    #[test]
    fn not_adjacent_empty() {
        // sou-nanda/not-adjacent: NIL.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![2137720])]);
        let r = lite_sl_owned(3, 6, vec![seg(3, 6, vec![2140410])]);
        assert!(synergy_sou_nanda(&l, &r).is_empty());
    }

    #[test]
    fn left_miss_empty() {
        // sou-nanda/left-miss: NIL.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![99])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![2140410])]);
        assert!(synergy_sou_nanda(&l, &r).is_empty());
    }
}

mod synergy_no_adjectives {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_synergies.lisp on .103, 2026-05-18).

    #[test]
    fn positive_kpcl_k() {
        // no-adj/positive-k: l adj-no with k=T, r seq 1469800.
        // RIGHT-SL start=1 end=2 segs=1, SYNERGY desc="no-adjective"
        // conn=" " score=15 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((true, false, false, false), vec!["adj-no"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![1469800])],
        );
        let got = synergy_no_adjectives(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 2);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("no-adjective"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_kpcl_l() {
        // no-adj/positive-l: l=T satisfies (or k l (and p c)).
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, true), vec!["adj-no"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![1469800])],
        );
        let got = synergy_no_adjectives(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 15);
    }

    #[test]
    fn positive_kpcl_pc() {
        // no-adj/positive-pc: (and p c) satisfies the test.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, true, true, false), vec!["adj-no"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![1469800])],
        );
        let got = synergy_no_adjectives(&l, &r);
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn neg_kpcl_all_nil() {
        // no-adj/neg-kpcl-all-nil: NIL.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, false), vec!["adj-no"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![1469800])],
        );
        assert!(synergy_no_adjectives(&l, &r).is_empty());
    }

    #[test]
    fn neg_kpcl_p_only() {
        // no-adj/neg-p-only: p without c, no k, no l -> kpcl-test false.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, true, false, false), vec!["adj-no"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![1469800])],
        );
        assert!(synergy_no_adjectives(&l, &r).is_empty());
    }

    #[test]
    fn neg_wrong_posi() {
        // no-adj/neg-no-posi: posi=("n"), not adj-no.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![1469800])],
        );
        assert!(synergy_no_adjectives(&l, &r).is_empty());
    }
}

mod synergy_na_adjectives {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_synergies.lisp on .103, 2026-05-18).

    #[test]
    fn positive_na() {
        // na-adj/positive-na: l adj-na with k=T, r seq 2029110.
        // RIGHT-SL start=2 end=3 segs=1, SYNERGY desc="na-adjective"
        // conn=" " score=15 start=2 end=2, LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg((true, false, false, false), vec!["adj-na"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg((false, false, false, false), vec![], vec![2029110])],
        );
        let got = synergy_na_adjectives(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("na-adjective"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_ni() {
        // na-adj/positive-ni: l adj-na with l=T, r seq 2028990 (に).
        let l = lite_sl_owned(
            0,
            2,
            vec![seg((false, false, false, true), vec!["adj-na"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg((false, false, false, false), vec![], vec![2028990])],
        );
        let got = synergy_na_adjectives(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 15);
        assert_eq!(got[0].1.start, 2);
        assert_eq!(got[0].1.end, 2);
    }

    #[test]
    fn wrong_posi_empty() {
        // na-adj/neg-wrong-posi: l posi=("v5k"), not adj-na -> NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg((true, false, false, false), vec!["v5k"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg((false, false, false, false), vec![], vec![2029110])],
        );
        assert!(synergy_na_adjectives(&l, &r).is_empty());
    }
}

mod synergy_to_adverbs {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_449_451.lisp on .103, 2026-05-18).

    #[test]
    fn positive_k_span2() {
        // to-adv/positive-k: l adv-to k=T span=2 -> score = 10 + 10*2 = 30.
        // RIGHT-SL start=2 end=3 segs=1, SYNERGY desc="to-adverb"
        // conn=" " score=30 start=2 end=2, LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg((true, false, false, false), vec!["adv-to"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg((false, false, false, false), vec![], vec![1008490])],
        );
        let got = synergy_to_adverbs(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("to-adverb"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 30);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_l_span1() {
        // to-adv/positive-l: l=T span=1 -> score = 20.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, true), vec!["adv-to"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![1008490])],
        );
        let got = synergy_to_adverbs(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 20);
        assert_eq!(got[0].1.start, 1);
        assert_eq!(got[0].1.end, 1);
    }

    #[test]
    fn positive_p_alone_span3() {
        // to-adv/positive-p-alone: p=T c=NIL span=3 -> score = 40. Bare
        // `p` is the divergence vs synergy-no-adjectives / synergy-na-
        // adjectives whose kpcl-test is `(or k l (and p c))`.
        let l = lite_sl_owned(
            0,
            3,
            vec![seg((false, true, false, false), vec!["adv-to"], vec![])],
        );
        let r = lite_sl_owned(
            3,
            4,
            vec![seg((false, false, false, false), vec![], vec![1008490])],
        );
        let got = synergy_to_adverbs(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 40);
        assert_eq!(got[0].1.start, 3);
        assert_eq!(got[0].1.end, 3);
    }

    #[test]
    fn positive_p_and_c_span4() {
        // to-adv/positive-p-and-c: p=T c=T span=4 -> score = 50.
        let l = lite_sl_owned(
            0,
            4,
            vec![seg((false, true, true, false), vec!["adv-to"], vec![])],
        );
        let r = lite_sl_owned(
            4,
            5,
            vec![seg((false, false, false, false), vec![], vec![1008490])],
        );
        let got = synergy_to_adverbs(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 50);
        assert_eq!(got[0].1.start, 4);
        assert_eq!(got[0].1.end, 4);
    }

    #[test]
    fn positive_k_span1() {
        // to-adv/positive-span1: k=T span=1 -> score = 20.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((true, false, false, false), vec!["adv-to"], vec![])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((false, false, false, false), vec![], vec![1008490])],
        );
        let got = synergy_to_adverbs(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 20);
    }

    #[test]
    fn neg_kpcl_all_nil() {
        // to-adv/neg-kpcl-all-nil: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg((false, false, false, false), vec!["adv-to"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg((false, false, false, false), vec![], vec![1008490])],
        );
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }

    #[test]
    fn neg_c_alone() {
        // to-adv/neg-c-alone: c=T only (no k, no l, no p) — kpcl-test is
        // `(or k l p)` so bare c does not pass.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg((false, false, true, false), vec!["adv-to"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg((false, false, false, false), vec![], vec![1008490])],
        );
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }

    #[test]
    fn neg_wrong_posi() {
        // to-adv/neg-wrong-posi: posi=("n"), not adv-to -> NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg((false, false, false, false), vec![], vec![1008490])],
        );
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }

    #[test]
    fn neg_wrong_right_seq() {
        // to-adv/neg-wrong-right: r seq not 1008490.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg((true, false, false, false), vec!["adv-to"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg((false, false, false, false), vec![], vec![9999])],
        );
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }

    #[test]
    fn neg_non_adjacent() {
        // to-adv/neg-non-adjacent: l.end /= r.start.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg((true, false, false, false), vec!["adv-to"], vec![])],
        );
        let r = lite_sl_owned(
            5,
            6,
            vec![seg((false, false, false, false), vec![], vec![1008490])],
        );
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }

    #[test]
    fn neg_empty_left() {
        // to-adv/neg-empty-left: l segs empty.
        let l = lite_sl_owned(0, 2, vec![]);
        let r = lite_sl_owned(
            2,
            3,
            vec![seg((false, false, false, false), vec![], vec![1008490])],
        );
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }
}

mod synergy_suffix_chu {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
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

    fn seg(
        start: usize,
        end: usize,
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start,
            end,
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_442_448.lisp on .103, 2026-05-18).

    #[test]
    fn positive_1620400() {
        // suffix-chu/positive-1620400: RIGHT-SL start=2 end=3 segs=1,
        // SYN desc="suffix-chu" conn="-" score=12 start=2 end=2,
        // LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(
                2,
                3,
                (false, false, false, false),
                vec![],
                vec![1620400],
            )],
        );
        let got = synergy_suffix_chu(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("suffix-chu"));
        assert_eq!(syn.connector.as_deref(), Some("-"));
        assert_eq!(syn.score, 12);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_2083570() {
        // suffix-chu/positive-2083570: same shape as positive_1620400.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(
                2,
                3,
                (false, false, false, false),
                vec![],
                vec![2083570],
            )],
        );
        let got = synergy_suffix_chu(&l, &r);
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn right_miss_empty() {
        // suffix-chu/right-miss: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(2, 3, (false, false, false, false), vec![], vec![99])],
        );
        assert!(synergy_suffix_chu(&l, &r).is_empty());
    }

    #[test]
    fn left_not_noun_empty() {
        // suffix-chu/left-not-noun: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["v5k"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(
                2,
                3,
                (false, false, false, false),
                vec![],
                vec![1620400],
            )],
        );
        assert!(synergy_suffix_chu(&l, &r).is_empty());
    }

    #[test]
    fn not_adjacent_empty() {
        // suffix-chu/not-adjacent: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            4,
            5,
            vec![seg(
                4,
                5,
                (false, false, false, false),
                vec![],
                vec![1620400],
            )],
        );
        assert!(synergy_suffix_chu(&l, &r).is_empty());
    }
}

mod synergy_suffix_tachi {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
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

    fn seg(
        start: usize,
        end: usize,
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start,
            end,
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_442_448.lisp on .103, 2026-05-18).

    #[test]
    fn positive() {
        // suffix-tachi/positive: RIGHT-SL start=2 end=3 segs=1,
        // SYN desc="suffix-tachi" conn="-" score=10 start=2 end=2,
        // LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(
                2,
                3,
                (false, false, false, false),
                vec![],
                vec![1416220],
            )],
        );
        let got = synergy_suffix_tachi(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("suffix-tachi"));
        assert_eq!(syn.connector.as_deref(), Some("-"));
        assert_eq!(syn.score, 10);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn right_miss_empty() {
        // suffix-tachi/right-miss: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(2, 3, (false, false, false, false), vec![], vec![99])],
        );
        assert!(synergy_suffix_tachi(&l, &r).is_empty());
    }

    #[test]
    fn left_not_noun_empty() {
        // suffix-tachi/left-not-noun: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["v5k"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(
                2,
                3,
                (false, false, false, false),
                vec![],
                vec![1416220],
            )],
        );
        assert!(synergy_suffix_tachi(&l, &r).is_empty());
    }

    #[test]
    fn not_adjacent_empty() {
        // suffix-tachi/not-adjacent: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            4,
            5,
            vec![seg(
                4,
                5,
                (false, false, false, false),
                vec![],
                vec![1416220],
            )],
        );
        assert!(synergy_suffix_tachi(&l, &r).is_empty());
    }

    #[test]
    fn multi_segs_partial_filter() {
        // suffix-tachi/multi-segs: l has 2 segs (one noun, one not),
        // r has 2 segs (one matches 1416220, one not). RIGHT-SL segs=1,
        // LEFT-SL segs=1.
        let l = lite_sl_owned(
            0,
            2,
            vec![
                seg(0, 2, (true, false, false, false), vec!["n"], vec![]),
                seg(0, 2, (true, false, false, false), vec!["v5k"], vec![]),
            ],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![
                seg(2, 3, (false, false, false, false), vec![], vec![1416220]),
                seg(2, 3, (false, false, false, false), vec![], vec![99]),
            ],
        );
        let got = synergy_suffix_tachi(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, _syn, left_sl) = &got[0];
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(left_sl.segments.len(), 1);
    }
}

mod synergy_suffix_buri {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
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

    fn seg(
        start: usize,
        end: usize,
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start,
            end,
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_442_448.lisp on .103, 2026-05-18).

    #[test]
    fn positive() {
        // suffix-buri/positive: RIGHT-SL start=2 end=4 segs=1,
        // SYN desc="suffix-buri" conn="" score=40 start=2 end=2,
        // LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            4,
            vec![seg(
                2,
                4,
                (false, false, false, false),
                vec![],
                vec![1361140],
            )],
        );
        let got = synergy_suffix_buri(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 4);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("suffix-buri"));
        assert_eq!(syn.connector.as_deref(), Some(""));
        assert_eq!(syn.score, 40);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn left_not_noun_empty() {
        // suffix-buri/left-not-noun: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["v5k"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            4,
            vec![seg(
                2,
                4,
                (false, false, false, false),
                vec![],
                vec![1361140],
            )],
        );
        assert!(synergy_suffix_buri(&l, &r).is_empty());
    }

    #[test]
    fn right_miss_empty() {
        // suffix-buri/right-miss: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            4,
            vec![seg(2, 4, (false, false, false, false), vec![], vec![99])],
        );
        assert!(synergy_suffix_buri(&l, &r).is_empty());
    }

    #[test]
    fn not_adjacent_empty() {
        // suffix-buri/not-adjacent: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            5,
            7,
            vec![seg(
                5,
                7,
                (false, false, false, false),
                vec![],
                vec![1361140],
            )],
        );
        assert!(synergy_suffix_buri(&l, &r).is_empty());
    }
}

mod synergy_suffix_sei {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
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

    fn seg(
        start: usize,
        end: usize,
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start,
            end,
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_442_448.lisp on .103, 2026-05-18).

    #[test]
    fn positive() {
        // suffix-sei/positive: RIGHT-SL start=2 end=3 segs=1,
        // SYN desc="suffix-sei" conn="" score=12 start=2 end=2,
        // LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(
                2,
                3,
                (false, false, false, false),
                vec![],
                vec![1375260],
            )],
        );
        let got = synergy_suffix_sei(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("suffix-sei"));
        assert_eq!(syn.connector.as_deref(), Some(""));
        assert_eq!(syn.score, 12);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn left_not_noun_empty() {
        // suffix-sei/left-not-noun: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["v5k"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(
                2,
                3,
                (false, false, false, false),
                vec![],
                vec![1375260],
            )],
        );
        assert!(synergy_suffix_sei(&l, &r).is_empty());
    }

    #[test]
    fn right_miss_empty() {
        // suffix-sei/right-miss: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(2, 3, (false, false, false, false), vec![], vec![99])],
        );
        assert!(synergy_suffix_sei(&l, &r).is_empty());
    }

    #[test]
    fn not_adjacent_empty() {
        // suffix-sei/not-adjacent: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            4,
            5,
            vec![seg(
                4,
                5,
                (false, false, false, false),
                vec![],
                vec![1375260],
            )],
        );
        assert!(synergy_suffix_sei(&l, &r).is_empty());
    }
}

mod synergy_o_prefix {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_437_441.lisp on .103, 2026-05-18).

    #[test]
    fn positive_k() {
        // o-prefix/positive-k: l seq 1270190 (お), r kpcl k=T posi=("n").
        // RIGHT-SL start=1 end=2 segs=1, SYNERGY desc="o+noun"
        // conn="" score=10 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, false), vec![], vec![1270190])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let got = synergy_o_prefix(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 2);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("o+noun"));
        assert_eq!(syn.connector.as_deref(), Some(""));
        assert_eq!(syn.score, 10);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_l() {
        // o-prefix/positive-l: r kpcl l=T, kpcl-test (or k l) satisfied.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, false), vec![], vec![1270190])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((false, false, false, true), vec!["n"], vec![])],
        );
        let got = synergy_o_prefix(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 10);
    }

    #[test]
    fn neg_pc_only() {
        // o-prefix/neg-pc-only: kpcl-test is (or k l), NOT (and p c) — NIL.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, false), vec![], vec![1270190])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((false, true, true, false), vec!["n"], vec![])],
        );
        assert!(synergy_o_prefix(&l, &r).is_empty());
    }

    #[test]
    fn neg_no_n_posi() {
        // o-prefix/neg-no-n-posi: posi=("adj-na"), not "n" — NIL.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, false), vec![], vec![1270190])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((true, false, false, false), vec!["adj-na"], vec![])],
        );
        assert!(synergy_o_prefix(&l, &r).is_empty());
    }

    #[test]
    fn neg_left_miss() {
        // o-prefix/neg-left-miss: l seq doesn't match 1270190 — NIL.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, false), vec![], vec![9999999])],
        );
        let r = lite_sl_owned(
            1,
            2,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        assert!(synergy_o_prefix(&l, &r).is_empty());
    }
}

mod synergy_kanji_prefix {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_synergies.lisp on .103, 2026-05-18).

    #[test]
    fn positive_mi() {
        // kanji-prefix/positive-mi: l seq 2242840, r k=T posi=("n").
        // RIGHT-SL start=1 end=3 segs=1, SYNERGY desc="kanji prefix+noun"
        // conn="" score=15 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, false), vec![], vec![2242840])],
        );
        let r = lite_sl_owned(
            1,
            3,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let got = synergy_kanji_prefix(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("kanji prefix+noun"));
        assert_eq!(syn.connector.as_deref(), Some(""));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_fu() {
        // kanji-prefix/positive-fu: l seq 1922780.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, false), vec![], vec![1922780])],
        );
        let r = lite_sl_owned(
            1,
            3,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let got = synergy_kanji_prefix(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 15);
    }

    #[test]
    fn positive_2423740() {
        // kanji-prefix/positive-2423740: l seq 2423740.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, false), vec![], vec![2423740])],
        );
        let r = lite_sl_owned(
            1,
            3,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        let got = synergy_kanji_prefix(&l, &r);
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn neg_no_k() {
        // kanji-prefix/neg-no-k: r kpcl k=NIL even with posi=("n") -> NIL.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, false), vec![], vec![2242840])],
        );
        let r = lite_sl_owned(
            1,
            3,
            vec![seg((false, false, false, true), vec!["n"], vec![])],
        );
        assert!(synergy_kanji_prefix(&l, &r).is_empty());
    }

    #[test]
    fn neg_no_n_posi() {
        // kanji-prefix/neg-no-n-posi: r k=T but posi=("v5k") (not "n").
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, false), vec![], vec![2242840])],
        );
        let r = lite_sl_owned(
            1,
            3,
            vec![seg((true, false, false, false), vec!["v5k"], vec![])],
        );
        assert!(synergy_kanji_prefix(&l, &r).is_empty());
    }

    #[test]
    fn neg_left_miss() {
        // kanji-prefix/neg-left-miss: l seq 9999 doesn't match.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg((false, false, false, false), vec![], vec![9999])],
        );
        let r = lite_sl_owned(
            1,
            3,
            vec![seg((true, false, false, false), vec!["n"], vec![])],
        );
        assert!(synergy_kanji_prefix(&l, &r).is_empty());
    }
}

mod synergy_shicha_ikenai {
    use crate::dict::compound_text_class::{CompoundText, ScoreMod};
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::grammar::synergy::*;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::segment_list_struct::SegmentList;
    use crate::dict::segment_struct::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::simple_text_class::SimpleText;

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

    fn compound_word(child_seqs: &[i32]) -> KaniWordDispatchEnum {
        let words: Vec<KaniWordDispatchEnum> = child_seqs
            .iter()
            .enumerate()
            .map(|(i, s)| KaniWordDispatchEnum::Kana(kana(&format!("w{i}"), *s)))
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

    fn dummy_kana() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(kana("x", 0))
    }

    fn seg(start: usize, end: usize, word: KaniWordDispatchEnum, seq_set: Vec<i32>) -> Segment {
        Segment {
            start,
            end,
            word,
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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_442_448.lisp on .103, 2026-05-18).

    #[test]
    fn positive() {
        // shicha-ikenai/positive: RIGHT-SL start=3 end=7 segs=1,
        // SYN desc="shicha ikenai" conn=" " score=50 start=3 end=3,
        // LEFT-SL start=0 end=3 segs=1.
        let l = lite_sl_owned(
            0,
            3,
            vec![seg(0, 3, compound_word(&[1234567, 2028920]), vec![])],
        );
        let r = lite_sl_owned(3, 7, vec![seg(3, 7, dummy_kana(), vec![1612750])]);
        let got = synergy_shicha_ikenai(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 3);
        assert_eq!(right_sl.end, 7);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("shicha ikenai"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 50);
        assert_eq!(syn.start, 3);
        assert_eq!(syn.end, 3);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 3);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn right_miss_empty() {
        // shicha-ikenai/right-miss: NIL.
        let l = lite_sl_owned(
            0,
            3,
            vec![seg(0, 3, compound_word(&[1234567, 2028920]), vec![])],
        );
        let r = lite_sl_owned(3, 7, vec![seg(3, 7, dummy_kana(), vec![99999])]);
        assert!(synergy_shicha_ikenai(&l, &r).is_empty());
    }

    #[test]
    fn not_adjacent_empty() {
        // shicha-ikenai/not-adjacent: NIL.
        let l = lite_sl_owned(
            0,
            3,
            vec![seg(0, 3, compound_word(&[1234567, 2028920]), vec![])],
        );
        let r = lite_sl_owned(5, 9, vec![seg(5, 9, dummy_kana(), vec![1612750])]);
        assert!(synergy_shicha_ikenai(&l, &r).is_empty());
    }

    #[test]
    fn left_not_compound_empty() {
        // shicha-ikenai/left-not-compound: NIL.
        // Simple word with seq 2028920 fails filter-is-compound-end.
        let l = lite_sl_owned(0, 1, vec![seg(0, 1, dummy_kana(), vec![2028920])]);
        let r = lite_sl_owned(1, 5, vec![seg(1, 5, dummy_kana(), vec![1612750])]);
        assert!(synergy_shicha_ikenai(&l, &r).is_empty());
    }
}

mod synergy_shika_negative {
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::conj_prop_dao::ConjProp;
    use crate::dict::grammar::synergy::*;
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

    fn prop(neg: Option<bool>) -> ConjProp {
        ConjProp {
            id: 0,
            conj_id: 0,
            pos: "v5k".into(),
            conj_type: 1,
            neg,
            fml: None,
        }
    }

    fn cdata(neg: Option<bool>) -> ConjData {
        ConjData {
            seq: Some(1),
            from: Some(2),
            via: None,
            prop: Some(prop(neg)),
            src_map: vec![],
        }
    }

    fn seg(start: usize, end: usize, seq_set: Vec<i32>, conj: Vec<ConjData>) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
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
            }),
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

    // REPL probes:
    // - /tmp/probe_442_448.lisp on .103, 2026-05-18: t / nil cases.
    // - /tmp/probe_shika_null.lisp on .103: :NULL case (DB-null neg
    //   keyword is truthy in CL, so synergy fires).
    //
    // Rust ↔ Lisp neg mapping (parse_opt_bool, audit/common/mod.rs:1789):
    //   Some(true)  ↔ Lisp t      → FIRE
    //   Some(false) ↔ Lisp nil    → reject
    //   None        ↔ Lisp :NULL  → FIRE (:NULL is a truthy keyword)

    #[test]
    fn positive_neg_t() {
        // shika-negative/positive (neg=t): RIGHT-SL start=2 end=5 segs=1,
        // SYN desc="shika+neg" conn=" " score=50 start=2 end=2,
        // LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![], vec![cdata(Some(true))])]);
        let got = synergy_shika_negative(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 5);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("shika+neg"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 50);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_neg_null() {
        // shika-negative/neg=:NULL ALONE -- expect FIRE (REPL
        // /tmp/probe_shika_null.lisp: COUNT=1 desc="shika+neg"
        // score=50). :NULL keyword is truthy in CL, so `some` returns
        // truthy and the filter accepts.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![], vec![cdata(None)])]);
        let got = synergy_shika_negative(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.description.as_deref(), Some("shika+neg"));
        assert_eq!(got[0].1.score, 50);
    }

    #[test]
    fn right_neg_nil_empty() {
        // shika-negative/neg=NIL ALONE -- expect NIL (REPL
        // /tmp/probe_shika_null.lisp). Lisp nil is the sole falsy
        // value, so `some` returns nil and the filter rejects.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![], vec![cdata(Some(false))])]);
        assert!(synergy_shika_negative(&l, &r).is_empty());
    }

    #[test]
    fn right_empty_conj_empty() {
        // shika-negative/right-empty-conj: NIL.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![], vec![])]);
        assert!(synergy_shika_negative(&l, &r).is_empty());
    }

    #[test]
    fn left_miss_empty() {
        // shika-negative/left-miss: NIL.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![99], vec![])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![], vec![cdata(Some(true))])]);
        assert!(synergy_shika_negative(&l, &r).is_empty());
    }

    #[test]
    fn not_adjacent_empty() {
        // shika-negative/not-adjacent: NIL.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(4, 7, vec![seg(4, 7, vec![], vec![cdata(Some(true))])]);
        assert!(synergy_shika_negative(&l, &r).is_empty());
    }

    #[test]
    fn multi_conj_nil_plus_nil_empty() {
        // shika-negative/neg=NIL+NIL -- expect NIL (REPL probe).
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(
            2,
            5,
            vec![seg(
                2,
                5,
                vec![],
                vec![cdata(Some(false)), cdata(Some(false))],
            )],
        );
        assert!(synergy_shika_negative(&l, &r).is_empty());
    }

    #[test]
    fn multi_conj_nil_plus_null_fires() {
        // shika-negative/neg=NIL+:NULL -- expect FIRE (REPL probe).
        // The :NULL cdata's truthy neg-value satisfies `some`.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(
            2,
            5,
            vec![seg(2, 5, vec![], vec![cdata(Some(false)), cdata(None)])],
        );
        let got = synergy_shika_negative(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 50);
    }

    #[test]
    fn multi_conj_nil_plus_t_fires() {
        // shika-negative/multi-conj-mixed: RIGHT-SL segs=1, LEFT-SL segs=1.
        // Mirrors the original probe_442_448 test but with the
        // corrected nil mapping (Some(false), not None).
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(
            2,
            5,
            vec![seg(
                2,
                5,
                vec![],
                vec![cdata(Some(false)), cdata(Some(true))],
            )],
        );
        let got = synergy_shika_negative(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, _syn, left_sl) = &got[0];
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(left_sl.segments.len(), 1);
    }
}
