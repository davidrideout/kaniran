//! Port of `ichiran/dict:penalty-short` (`dict-grammar.lisp:996-1001`).
//!
//! ```lisp
//! (def-generic-penalty penalty-short (l r)
//!   (filter-short-kana 1)
//!   (filter-short-kana 1 :except '("と"))
//!   :description "short"
//!   :serial nil
//!   :score -9)
//! ```

use super::def_generic_penalty_macro::{def_generic_penalty_body, DefGenericPenaltyOpts};
use super::filter_short_kana::filter_short_kana;
use super::kani_lite_segment_list::KaniLiteSegmentList;
use super::synergy_struct::Synergy;

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
