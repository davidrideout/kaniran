//! Port of `ichiran/dict:synergy-sou-nanda` (`dict-grammar.lisp:856`).
//!
//! ```lisp
//! ;; TODO: remove this hack
//! (def-generic-synergy synergy-sou-nanda (l r)
//!   (filter-in-seq-set 2137720)
//!   (filter-in-seq-set 2140410)
//!   :description "sou na n da"
//!   :score 50
//!   :connector " ")
//! ```
//!
//! Divergences from Lisp:
//! - `pushnew ',name *synergy-list*` from the `defsynergy` expansion
//!   moves to the `*synergy-list*` port (separate wave).

use std::sync::Arc;

use super::def_generic_synergy_macro::{def_generic_synergy_body, DefGenericSynergyOpts};
use super::filter_in_seq_set::filter_in_seq_set;
use super::kani_lite_segment_list::KaniLiteSegmentList;
use super::synergy_struct::Synergy;

pub fn synergy_sou_nanda(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_in_seq_set(vec![2137720]),
        filter_in_seq_set(vec![2140410]),
        &DefGenericSynergyOpts {
            description: Some("sou na n da"),
            connector: " ",
            score: 50,
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
