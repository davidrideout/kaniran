//! Port of `ichiran/dict:synergy-suffix-chu` (`dict-grammar.lisp:884`).
//!
//! ```lisp
//! (def-generic-synergy synergy-suffix-chu (l r)
//!   #'filter-is-noun
//!   (filter-in-seq-set 1620400 2083570)
//!   :description "suffix-chu"
//!   :score 12
//!   :connector "-")
//! ```
//!
//! Divergences from Lisp:
//! - `pushnew ',name *synergy-list*` from the `defsynergy` expansion
//!   moves to the `*synergy-list*` port (separate wave).

use std::sync::Arc;

use super::def_generic_synergy_macro::{def_generic_synergy_body, DefGenericSynergyOpts};
use super::filter_in_seq_set::filter_in_seq_set;
use super::filter_is_noun::filter_is_noun;
use super::kani::KaniLiteSegmentList;
use super::synergy_struct::Synergy;

pub fn synergy_suffix_chu(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(vec![1620400, 2083570]),
        &DefGenericSynergyOpts {
            description: Some("suffix-chu"),
            connector: "-",
            score: 12,
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
            vec![seg(2, 3, (false, false, false, false), vec![], vec![1620400])],
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
            vec![seg(2, 3, (false, false, false, false), vec![], vec![2083570])],
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
            vec![seg(2, 3, (false, false, false, false), vec![], vec![1620400])],
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
            vec![seg(4, 5, (false, false, false, false), vec![], vec![1620400])],
        );
        assert!(synergy_suffix_chu(&l, &r).is_empty());
    }
}
