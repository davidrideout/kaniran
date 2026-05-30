//! Port of `ichiran/dict:penalty-semi-final` (`dict-grammar.lisp:1003-1009`).
//!
//! ```lisp
//! (def-generic-penalty penalty-semi-final (l r)
//!   (lambda (sl)
//!     (some (lambda (s) (funcall (apply 'filter-in-seq-set *semi-final-prt*) s))
//!           (segment-list-segments sl)))
//!   (constantly t)
//!   :description "semi-final not final"
//!   :score -15)
//! ```

use super::_star_semi_final_prt_star_::semi_final_prt;
use super::def_generic_penalty_macro::{def_generic_penalty_body, DefGenericPenaltyOpts};
use super::filter_in_seq_set::filter_in_seq_set;
use super::kani::KaniLiteSegmentList;
use super::synergy_struct::Synergy;

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
