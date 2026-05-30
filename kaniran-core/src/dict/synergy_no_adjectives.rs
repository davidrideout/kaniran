//! Port of `ichiran/dict:synergy-no-adjectives` (`dict-grammar.lisp:863`).
//!
//! ```lisp
//! (def-generic-synergy synergy-no-adjectives (l r)
//!   (filter-is-pos ("adj-no") (segment k p c l) (or k l (and p c)))
//!   (filter-in-seq-set 1469800) ;; の
//!   :description "no-adjective"
//!   :score 15
//!   :connector " ")
//! ```
//!
//! Divergences from Lisp:
//! - The `filter-is-pos` filter (`dict-grammar.lisp:864`,
//!   `(or k l (and p c))`) is built via [`filter_is_pos`].
//! - `pushnew ',name *synergy-list*` from the `defsynergy` expansion
//!   moves to the `*synergy-list*` port (separate wave).

use std::sync::Arc;

use super::def_generic_synergy_macro::{def_generic_synergy_body, DefGenericSynergyOpts};
use super::filter_in_seq_set::filter_in_seq_set;
use super::filter_is_pos_macro::filter_is_pos;
use super::kani::POS_ADJ_NO;
use super::kani::KaniLiteSegmentList;
use super::synergy_struct::Synergy;

pub fn synergy_no_adjectives(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        // dict-grammar.lisp:864 (filter-is-pos ("adj-no") (or k l (and p c)))
        filter_is_pos(POS_ADJ_NO, |k, p, c, l| k || l || (p && c)),
        filter_in_seq_set(vec![1469800]),
        &DefGenericSynergyOpts {
            description: Some("no-adjective"),
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

    // REPL probes (/tmp/probe_synergies.lisp on .103, 2026-05-18).

    #[test]
    fn positive_kpcl_k() {
        // no-adj/positive-k: l adj-no with k=T, r seq 1469800.
        // RIGHT-SL start=1 end=2 segs=1, SYNERGY desc="no-adjective"
        // conn=" " score=15 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["adj-no"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        let got = synergy_no_adjectives(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 2);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("no-adjective"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_kpcl_l() {
        // no-adj/positive-l: l=T satisfies (or k l (and p c)).
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, true), vec!["adj-no"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        let got = synergy_no_adjectives(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 15);
    }

    #[test]
    fn positive_kpcl_pc() {
        // no-adj/positive-pc: (and p c) satisfies the test.
        let l = lite_sl_owned(0, 1, vec![seg((false, true, true, false), vec!["adj-no"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        let got = synergy_no_adjectives(&l, &r);
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn neg_kpcl_all_nil() {
        // no-adj/neg-kpcl-all-nil: NIL.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec!["adj-no"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        assert!(synergy_no_adjectives(&l, &r).is_empty());
    }

    #[test]
    fn neg_kpcl_p_only() {
        // no-adj/neg-p-only: p without c, no k, no l -> kpcl-test false.
        let l = lite_sl_owned(0, 1, vec![seg((false, true, false, false), vec!["adj-no"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        assert!(synergy_no_adjectives(&l, &r).is_empty());
    }

    #[test]
    fn neg_wrong_posi() {
        // no-adj/neg-no-posi: posi=("n"), not adj-no.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        assert!(synergy_no_adjectives(&l, &r).is_empty());
    }
}
