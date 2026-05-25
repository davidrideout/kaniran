//! Port of `ichiran/dict:synergy-noun-da` (`dict-grammar.lisp:841`).
//!
//! ```lisp
//! (def-generic-synergy synergy-noun-da (l r)
//!   #'filter-is-noun
//!   (filter-in-seq-set 2089020) ;; だ
//!   :description "noun+da"
//!   :score 10
//!   :connector " ")
//! ```
//!
//! Divergences from Lisp:
//! - `pushnew ',name *synergy-list*` from the `defsynergy` expansion
//!   moves to the `*synergy-list*` port (separate wave).

use std::sync::Arc;

use super::def_generic_synergy_macro::{def_generic_synergy_body, DefGenericSynergyOpts};
use super::filter_in_seq_set::filter_in_seq_set;
use super::filter_is_noun::filter_is_noun;
use super::kani_lite_segment_list::KaniLiteSegmentList;
use super::synergy_struct::Synergy;

pub fn synergy_noun_da(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(vec![2089020]),
        &DefGenericSynergyOpts {
            description: Some("noun+da"),
            connector: " ",
            score: 10,
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
    fn positive() {
        // noun-da/positive: l noun (kpcl k=T posi=("n")), r seq 2089020 (だ).
        // RIGHT-SL start=1 end=2 segs=1, SYNERGY desc="noun+da"
        // conn=" " score=10 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![2089020])]);
        let got = synergy_noun_da(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 2);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("noun+da"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 10);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn not_adjacent_empty() {
        // noun-da/not-adjacent: NIL.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(5, 6, vec![seg((false, false, false, false), vec![], vec![2089020])]);
        assert!(synergy_noun_da(&l, &r).is_empty());
    }

    #[test]
    fn left_not_noun_empty() {
        // noun-da/left-not-noun: NIL.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["v5k"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![2089020])]);
        assert!(synergy_noun_da(&l, &r).is_empty());
    }

    #[test]
    fn right_misses_empty() {
        // noun-da/right-misses: NIL.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![9999999])]);
        assert!(synergy_noun_da(&l, &r).is_empty());
    }
}
