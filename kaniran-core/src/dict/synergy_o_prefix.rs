//! Port of `ichiran/dict:synergy-o-prefix` (`dict-grammar.lisp:913`).
//!
//! ```lisp
//! (def-generic-synergy synergy-o-prefix (l r)
//!   (filter-in-seq-set 1270190)
//!   (filter-is-pos ("n") (segment k p c l) (or k l))
//!   :description "o+noun"
//!   :score 10
//!   :connector "")
//! ```
//!
//! Divergences from Lisp:
//! - The `filter-is-pos` macro expansion (`dict-grammar.lisp:757-764`)
//!   is inlined as a closure on `Segment`'s `kpcl` tuple and `posi`
//!   list per CONVENTIONS §4.6. The `kpcl-test` body is `(or k l)`.
//! - `pushnew ',name *synergy-list*` from the `defsynergy` expansion
//!   moves to the `*synergy-list*` port (separate wave).

use super::filter_in_seq_set::filter_in_seq_set;
use super::make_segment_list_from::make_segment_list_from;
use super::segment_list_struct::SegmentList;
use super::segment_struct::Segment;
use super::synergy_struct::Synergy;

pub fn synergy_o_prefix(
    l: &SegmentList,
    r: &SegmentList,
) -> Vec<(SegmentList, Synergy, SegmentList)> {
    let start = l.end;
    let end = r.start;
    // dict-grammar.lisp:731-746 (def-generic-synergy expansion)
    if start != end {
        return vec![];
    }
    let test_left = filter_in_seq_set(vec![1270190]);
    // dict-grammar.lisp:757-764 (filter-is-pos macro expansion)
    let test_right = |seg: &Segment| -> bool {
        let info = match &seg.info {
            Some(info) => info,
            None => return false,
        };
        let (k, _p, _c, lv) = info.kpcl;
        (k || lv) && info.posi.iter().any(|x| x == "n")
    };
    let left: Vec<_> = l.segments.iter().filter(|s| test_left(s)).cloned().collect();
    let right: Vec<_> = r.segments.iter().filter(|s| test_right(s)).cloned().collect();
    if left.is_empty() || right.is_empty() {
        return vec![];
    }
    let syn = Synergy {
        description: Some("o+noun".to_string()),
        connector: Some(String::new()),
        score: 10,
        start,
        end,
    };
    vec![(
        make_segment_list_from(r, right),
        syn,
        make_segment_list_from(l, left),
    )]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::segment_struct::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo};
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

    fn sl(start: usize, end: usize, segments: Vec<Segment>) -> SegmentList {
        SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }
    }

    // REPL probes (/tmp/probe_437_441.lisp on .103, 2026-05-18).

    #[test]
    fn positive_k() {
        // o-prefix/positive-k: l seq 1270190 (お), r kpcl k=T posi=("n").
        // RIGHT-SL start=1 end=2 segs=1, SYNERGY desc="o+noun"
        // conn="" score=10 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = sl(0, 1, vec![seg((false, false, false, false), vec![], vec![1270190])]);
        let r = sl(1, 2, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let got = synergy_o_prefix(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 2);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("o+noun"));
        assert_eq!(syn.connector.as_deref(), Some(""));
        assert_eq!(syn.score, 10);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_l() {
        // o-prefix/positive-l: r kpcl l=T, kpcl-test (or k l) satisfied.
        let l = sl(0, 1, vec![seg((false, false, false, false), vec![], vec![1270190])]);
        let r = sl(1, 2, vec![seg((false, false, false, true), vec!["n"], vec![])]);
        let got = synergy_o_prefix(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 10);
    }

    #[test]
    fn neg_pc_only() {
        // o-prefix/neg-pc-only: kpcl-test is (or k l), NOT (and p c) — NIL.
        let l = sl(0, 1, vec![seg((false, false, false, false), vec![], vec![1270190])]);
        let r = sl(1, 2, vec![seg((false, true, true, false), vec!["n"], vec![])]);
        assert!(synergy_o_prefix(&l, &r).is_empty());
    }

    #[test]
    fn neg_no_n_posi() {
        // o-prefix/neg-no-n-posi: posi=("adj-na"), not "n" — NIL.
        let l = sl(0, 1, vec![seg((false, false, false, false), vec![], vec![1270190])]);
        let r = sl(1, 2, vec![seg((true, false, false, false), vec!["adj-na"], vec![])]);
        assert!(synergy_o_prefix(&l, &r).is_empty());
    }

    #[test]
    fn neg_left_miss() {
        // o-prefix/neg-left-miss: l seq doesn't match 1270190 — NIL.
        let l = sl(0, 1, vec![seg((false, false, false, false), vec![], vec![9999999])]);
        let r = sl(1, 2, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        assert!(synergy_o_prefix(&l, &r).is_empty());
    }
}
