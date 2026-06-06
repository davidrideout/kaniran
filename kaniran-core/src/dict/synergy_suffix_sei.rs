//! Port of `ichiran/dict:synergy-suffix-sei` (`dict-grammar.lisp:905`).
//!
//! Scores a noun on the left followed by the 性 (-sei) suffix word.

use std::sync::Arc;

use super::def_generic_synergy_macro::{def_generic_synergy_body, DefGenericSynergyOpts};
use super::filter_in_seq_set::filter_in_seq_set;
use super::filter_is_noun::filter_is_noun;
use super::kani_lite_segment_list::KaniLiteSegmentList;
use super::synergy_struct::Synergy;

pub fn synergy_suffix_sei(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(vec![1375260]),
        &DefGenericSynergyOpts {
            description: Some("suffix-sei"),
            connector: "",
            score: 12,
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
            vec![seg(2, 3, (false, false, false, false), vec![], vec![1375260])],
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
            vec![seg(2, 3, (false, false, false, false), vec![], vec![1375260])],
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
            vec![seg(4, 5, (false, false, false, false), vec![], vec![1375260])],
        );
        assert!(synergy_suffix_sei(&l, &r).is_empty());
    }
}
