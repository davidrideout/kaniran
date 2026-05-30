//! Port of the dict-grammar.lisp filter layer.

use crate::dict::kani::{
    KaniLiteSegment, KaniLiteSegmentList, KPCL_C, KPCL_K, KPCL_L, KPCL_P, POS_NOUN,
};
use std::sync::Arc;

pub fn filter_is_noun(segment: &Arc<KaniLiteSegment>) -> bool {
    let kpcl = segment.kpcl;
    let kpcl_gate = (kpcl & (KPCL_L | KPCL_K)) != 0 || (kpcl & KPCL_P != 0 && kpcl & KPCL_C != 0);
    if kpcl_gate && (segment.pos & POS_NOUN) != 0 {
        return true;
    }
    segment.is_counter && !segment.seq_set.is_empty()
}

pub fn filter_is_pos(
    pos_mask: u16,
    kpcl_test: impl Fn(bool, bool, bool, bool) -> bool,
) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool {
        let k = segment.kpcl & KPCL_K != 0;
        let p = segment.kpcl & KPCL_P != 0;
        let c = segment.kpcl & KPCL_C != 0;
        let l = segment.kpcl & KPCL_L != 0;
        kpcl_test(k, p, c, l) && (segment.pos & pos_mask) != 0
    }
}

pub fn filter_in_seq_set(seqs: Vec<i32>) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool { seqs.iter().any(|s| segment.seq_set.contains(s)) }
}

pub fn filter_in_seq_set_simple(seqs: Vec<i32>) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool {
        segment.has_simple_seq && seqs.iter().any(|s| segment.seq_set.contains(s))
    }
}

pub fn filter_is_conjugation(conj_type: i32) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool { segment.conj_types.contains(&conj_type) }
}

pub static NOUN_PARTICLES: &[i32] = &[
    2028920, // は
    2028930, // が
    2028990, // に
    2028980, // で
    2029000, // へ
    1007340, // だけ
    1579080, // ごろ
    1525680, // まで
    2028940, // も
    1582300, // など
    2215430, // には
    1469800, // の
    1009990, // のみ
    2029010, // を
    1005120, // さえ
    2034520, // でさえ
    1005120, // すら
    1008490, // と
    1008530, // とか
    1008590, // として
    2028950, // とは
    2028960, // や
    1009600, // にとって
];

pub fn filter_is_compound_end(seqs: Vec<i32>) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool {
        match segment.compound_end_seq {
            Some(s) => seqs.contains(&s),
            None => false,
        }
    }
}

pub fn filter_is_compound_end_text(texts: Vec<String>) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool {
        match segment.compound_end_text.as_deref() {
            Some(end) => texts.iter().any(|t| t == end),
            None => false,
        }
    }
}

pub fn filter_short_kana(len: usize, except: Vec<String>) -> impl Fn(&KaniLiteSegmentList) -> bool {
    move |segment_list| -> bool {
        let seg = match segment_list.segments.first() {
            Some(s) => s,
            None => return false,
        };
        if segment_list.end - segment_list.start > len {
            return false;
        }
        if seg.kpcl & KPCL_K != 0 {
            return false;
        }
        if !except.is_empty() && except.iter().any(|e| e.as_str() == seg.text.as_ref()) {
            return false;
        }
        true
    }
}

pub fn classify<T, F>(filter: F, list: &[T]) -> (Vec<T>, Vec<T>)
where
    T: Clone,
    F: Fn(&T) -> bool,
{
    let mut yep: Vec<T> = Vec::new();
    let mut nope: Vec<T> = Vec::new();
    for element in list {
        if filter(element) {
            yep.push(element.clone());
        } else {
            nope.push(element.clone());
        }
    }
    (yep, nope)
}

#[cfg(test)]
mod test_filter_is_pos_macro {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::{
        KaniWordDispatchEnum, POS_ADJ_NA, POS_ADJ_NO, POS_ADV_TO, POS_CTR, POS_N,
    };
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

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

#[cfg(test)]
mod test_filter_in_seq_set {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

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

#[cfg(test)]
mod test_filter_short_kana {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::{
        KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment, SegmentList,
    };
    use crate::dict::text_classes::SimpleText;

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
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_b.lisp on .103, 2026-05-18).

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

#[cfg(test)]
mod test_classify {
    use super::*;

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
