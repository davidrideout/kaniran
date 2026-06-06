//! Port of `ichiran/dict:synergy-no-da` (`dict-grammar.lisp:848`).
//!
//! "no da/desu" synergy: binds の/ん (seqs 1469800/2139720) on the left to
//! だ/です/だろう (seqs 2089020/1007370/1928670) on the right.

use std::sync::Arc;

use super::def_generic_synergy_macro::{def_generic_synergy_body, DefGenericSynergyOpts};
use super::filter_in_seq_set::filter_in_seq_set;
use super::kani_lite_segment_list::KaniLiteSegmentList;
use super::synergy_struct::Synergy;

pub fn synergy_no_da(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_in_seq_set(vec![1469800, 2139720]),
        filter_in_seq_set(vec![2089020, 1007370, 1928670]),
        &DefGenericSynergyOpts {
            description: Some("no da/desu"),
            connector: " ",
            score: 15,
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
