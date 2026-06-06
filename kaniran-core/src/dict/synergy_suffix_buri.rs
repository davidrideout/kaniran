//! Port of `ichiran/dict:synergy-suffix-buri` (`dict-grammar.lisp:898`).
//!
//! "suffix-buri" synergy: binds a noun on the left to ぶり (seq 1361140)
//! on the right.

use std::sync::Arc;

use super::def_generic_synergy_macro::{def_generic_synergy_body, DefGenericSynergyOpts};
use super::filter_in_seq_set::filter_in_seq_set;
use super::filter_is_noun::filter_is_noun;
use super::kani_lite_segment_list::KaniLiteSegmentList;
use super::synergy_struct::Synergy;

pub fn synergy_suffix_buri(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(vec![1361140]),
        &DefGenericSynergyOpts {
            description: Some("suffix-buri"),
            connector: "",
            score: 40,
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
            vec![seg(2, 4, (false, false, false, false), vec![], vec![1361140])],
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
            vec![seg(2, 4, (false, false, false, false), vec![], vec![1361140])],
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
            vec![seg(5, 7, (false, false, false, false), vec![], vec![1361140])],
        );
        assert!(synergy_suffix_buri(&l, &r).is_empty());
    }
}
