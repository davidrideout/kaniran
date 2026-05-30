//! Port of `ichiran/dict:synergy-kanji-prefix` (`dict-grammar.lisp:920`).
//!
//! ```lisp
//! (def-generic-synergy synergy-kanji-prefix (l r)
//!   (filter-in-seq-set 2242840 1922780 2423740) ;; 未 不
//!   (filter-is-pos ("n") (segment k p c l) k)
//!   :description "kanji prefix+noun"
//!   :score 15
//!   :connector "")
//! ```
//!
//! Divergences from Lisp:
//! - The `filter-is-pos` filter (`dict-grammar.lisp:922`) is built via
//!   [`filter_is_pos`]; the kpcl-test body is `k` alone — only the
//!   kanji-or-katakana bit is consulted.
//! - `pushnew ',name *synergy-list*` from the `defsynergy` expansion
//!   moves to the `*synergy-list*` port (separate wave).

use std::sync::Arc;

use super::def_generic_synergy_macro::{def_generic_synergy_body, DefGenericSynergyOpts};
use super::filter_in_seq_set::filter_in_seq_set;
use super::filter_is_pos_macro::filter_is_pos;
use super::kani::POS_N;
use super::kani::KaniLiteSegmentList;
use super::synergy_struct::Synergy;

pub fn synergy_kanji_prefix(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_in_seq_set(vec![2242840, 1922780, 2423740]),
        // dict-grammar.lisp:922 (filter-is-pos ("n") k)
        filter_is_pos(POS_N, |k, _p, _c, _l| k),
        &DefGenericSynergyOpts {
            description: Some("kanji prefix+noun"),
            connector: "",
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
    fn positive_mi() {
        // kanji-prefix/positive-mi: l seq 2242840, r k=T posi=("n").
        // RIGHT-SL start=1 end=3 segs=1, SYNERGY desc="kanji prefix+noun"
        // conn="" score=15 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![2242840])]);
        let r = lite_sl_owned(1, 3, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let got = synergy_kanji_prefix(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("kanji prefix+noun"));
        assert_eq!(syn.connector.as_deref(), Some(""));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_fu() {
        // kanji-prefix/positive-fu: l seq 1922780.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![1922780])]);
        let r = lite_sl_owned(1, 3, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let got = synergy_kanji_prefix(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 15);
    }

    #[test]
    fn positive_2423740() {
        // kanji-prefix/positive-2423740: l seq 2423740.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![2423740])]);
        let r = lite_sl_owned(1, 3, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let got = synergy_kanji_prefix(&l, &r);
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn neg_no_k() {
        // kanji-prefix/neg-no-k: r kpcl k=NIL even with posi=("n") -> NIL.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![2242840])]);
        let r = lite_sl_owned(1, 3, vec![seg((false, false, false, true), vec!["n"], vec![])]);
        assert!(synergy_kanji_prefix(&l, &r).is_empty());
    }

    #[test]
    fn neg_no_n_posi() {
        // kanji-prefix/neg-no-n-posi: r k=T but posi=("v5k") (not "n").
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![2242840])]);
        let r = lite_sl_owned(1, 3, vec![seg((true, false, false, false), vec!["v5k"], vec![])]);
        assert!(synergy_kanji_prefix(&l, &r).is_empty());
    }

    #[test]
    fn neg_left_miss() {
        // kanji-prefix/neg-left-miss: l seq 9999 doesn't match.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![9999])]);
        let r = lite_sl_owned(1, 3, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        assert!(synergy_kanji_prefix(&l, &r).is_empty());
    }
}
