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
//!
//! Divergences from Lisp:
//! - Returns [`Option<Synergy>`] rather than `nil`-or-struct
//!   (CONVENTIONS §4.1).
//! - `pushnew ',name *penalty-list*` from the `defpenalty` expansion
//!   moves to the `*penalty-list*` port (separate wave).

use super::filter_short_kana::filter_short_kana;
use super::segment_list_struct::SegmentList;
use super::synergy_struct::Synergy;

pub fn penalty_short(l: &SegmentList, r: &SegmentList) -> Option<Synergy> {
    let start = l.end;
    let end = r.start;
    // dict-grammar.lisp:972-984 (def-generic-penalty expansion;
    // serial=nil elides the (= start end) guard, connector defaults to " ")
    let test_left = filter_short_kana(1, vec![]);
    let test_right = filter_short_kana(1, vec!["と".to_string()]);
    if !test_left(l) || !test_right(r) {
        return None;
    }
    Some(Synergy {
        description: Some("short".to_string()),
        connector: Some(" ".to_string()),
        score: -9,
        start,
        end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn sl(start: usize, end: usize, segments: Vec<Segment>) -> SegmentList {
        SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }
    }

    // REPL probes (/tmp/probe_b.lisp on .103, 2026-05-18).

    #[test]
    fn d1_both_spans_one_returns_synergy() {
        let l = sl(0, 1, vec![seg(0, 1, (false, false, false, false), "あ")]);
        let r = sl(3, 4, vec![seg(3, 4, (false, false, false, false), "い")]);
        let got = penalty_short(&l, &r).expect("synergy");
        assert_eq!(got.description.as_deref(), Some("short"));
        assert_eq!(got.connector.as_deref(), Some(" "));
        assert_eq!(got.score, -9);
        assert_eq!(got.start, 1);
        assert_eq!(got.end, 3);
    }

    #[test]
    fn d2_l_span_two_returns_none() {
        let l = sl(0, 2, vec![seg(0, 2, (false, false, false, false), "あい")]);
        let r = sl(2, 3, vec![seg(2, 3, (false, false, false, false), "い")]);
        assert!(penalty_short(&l, &r).is_none());
    }

    #[test]
    fn d3_r_text_in_r_except_returns_none() {
        let l = sl(0, 1, vec![seg(0, 1, (false, false, false, false), "あ")]);
        let r = sl(5, 6, vec![seg(5, 6, (false, false, false, false), "と")]);
        assert!(penalty_short(&l, &r).is_none());
    }

    #[test]
    fn d4_l_text_to_not_in_l_except_returns_synergy() {
        let l = sl(0, 1, vec![seg(0, 1, (false, false, false, false), "と")]);
        let r = sl(3, 4, vec![seg(3, 4, (false, false, false, false), "い")]);
        let got = penalty_short(&l, &r).expect("synergy");
        assert_eq!(got.score, -9);
    }

    #[test]
    fn d5_l_kpcl_first_set_returns_none() {
        let l = sl(0, 1, vec![seg(0, 1, (true, false, false, false), "あ")]);
        let r = sl(2, 3, vec![seg(2, 3, (false, false, false, false), "い")]);
        assert!(penalty_short(&l, &r).is_none());
    }

    #[test]
    fn d6_serial_nil_allows_non_adjacent() {
        let l = sl(0, 1, vec![seg(0, 1, (false, false, false, false), "あ")]);
        let r = sl(100, 101, vec![seg(100, 101, (false, false, false, false), "い")]);
        let got = penalty_short(&l, &r).expect("synergy");
        assert_eq!(got.start, 1);
        assert_eq!(got.end, 100);
    }

    #[test]
    fn d7_empty_l_segments_returns_none() {
        let l = sl(0, 1, vec![]);
        let r = sl(1, 2, vec![seg(1, 2, (false, false, false, false), "い")]);
        assert!(penalty_short(&l, &r).is_none());
    }
}
