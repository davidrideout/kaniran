//! Port of `ichiran/dict:synergy-oki` (`dict-grammar.lisp:951`).
//!
//! Synergy binding a counter (pos "ctr") on the left to おき
//! (seqs 2854117/2084550) on the right.

use std::sync::Arc;

use super::def_generic_synergy_macro::{def_generic_synergy_body, DefGenericSynergyOpts};
use super::filter_in_seq_set::filter_in_seq_set;
use super::filter_is_pos_macro::filter_is_pos;
use super::kani_lite_segment::POS_CTR;
use super::kani_lite_segment_list::KaniLiteSegmentList;
use super::synergy_struct::Synergy;

pub fn synergy_oki(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        // dict-grammar.lisp:952 (filter-is-pos ("ctr") t)
        filter_is_pos(POS_CTR, |_k, _p, _c, _l| true),
        filter_in_seq_set(vec![2854117, 2084550]),
        &DefGenericSynergyOpts {
            description: None,
            connector: "",
            score: 20,
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
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
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
    fn positive_2854117() {
        // oki/positive-2854117: l posi=("ctr") kpcl all nil, r seq 2854117.
        // RIGHT-SL start=1 end=3 segs=1, SYNERGY desc=NIL conn=""
        // score=20 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec!["ctr"], vec![])]);
        let r = lite_sl_owned(1, 3, vec![seg((false, false, false, false), vec![], vec![2854117])]);
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
        // oki/positive-2084550: r matches second seq in the set.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec!["ctr"], vec![])]);
        let r = lite_sl_owned(1, 3, vec![seg((false, false, false, false), vec![], vec![2084550])]);
        let got = synergy_oki(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 20);
    }

    #[test]
    fn neg_no_ctr_posi() {
        // oki/neg-no-ctr-posi: l posi=("n"), not "ctr" — NIL.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 3, vec![seg((false, false, false, false), vec![], vec![2854117])]);
        assert!(synergy_oki(&l, &r).is_empty());
    }

    #[test]
    fn neg_right_miss() {
        // oki/neg-right-miss: r seq doesn't match either — NIL.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["ctr"], vec![])]);
        let r = lite_sl_owned(1, 3, vec![seg((false, false, false, false), vec![], vec![9999999])]);
        assert!(synergy_oki(&l, &r).is_empty());
    }

    #[test]
    fn neg_not_adjacent() {
        // oki/neg-not-adjacent: l.end != r.start — NIL.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec!["ctr"], vec![])]);
        let r = lite_sl_owned(5, 7, vec![seg((false, false, false, false), vec![], vec![2854117])]);
        assert!(synergy_oki(&l, &r).is_empty());
    }
}
