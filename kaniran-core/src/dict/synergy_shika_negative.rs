//! Port of `ichiran/dict:synergy-shika-negative` (`dict-grammar.lisp:934`).
//!
//! ```lisp
//! (def-generic-synergy synergy-shika-negative (l r)
//!   (filter-in-seq-set 1005460) ;; しか
//!   (lambda (segment)
//!     (some (lambda (cdata)
//!             (conj-neg (conj-data-prop cdata)))
//!           (getf (segment-info segment) :conj)))
//!   :description "shika+neg"
//!   :score 50
//!   :connector " ")
//! ```
//!
//! Divergences from Lisp:
//! - `pushnew ',name *synergy-list*` from the `defsynergy` expansion
//!   moves to the `*synergy-list*` port (separate wave).
//! - The right-side `lambda` is inlined; `some` returns the first
//!   non-nil result from `(conj-neg (conj-data-prop cdata))`. Per
//!   `parse_opt_bool` in `audit/common/mod.rs`, Rust `Some(false)`
//!   mirrors Lisp `nil`, `None` mirrors Lisp `:NULL`, `Some(true)`
//!   mirrors Lisp `t`. Only Lisp `nil` is falsy — `:NULL` (DB-NULL
//!   keyword) is truthy in CL — so the predicate accepts any neg
//!   value other than `Some(false)`.

use super::conj_data_prop::conj_data_prop;
use super::filter_in_seq_set::filter_in_seq_set;
use super::make_segment_list_from::make_segment_list_from;
use super::segment_list_struct::SegmentList;
use super::segment_struct::Segment;
use super::synergy_struct::Synergy;

fn right_filter(segment: &Segment) -> bool {
    let info = match &segment.info {
        Some(info) => info,
        None => return false,
    };
    // dict-grammar.lisp:937 (`(some (lambda (cdata) (conj-neg (conj-data-prop cdata))) ...)`)
    // Lisp truthy = non-nil; only Some(false) (Lisp nil) rejects.
    info.conj
        .iter()
        .any(|cd| conj_data_prop(cd).is_some_and(|p| p.neg != Some(false)))
}

pub fn synergy_shika_negative(
    l: &SegmentList,
    r: &SegmentList,
) -> Vec<(SegmentList, Synergy, SegmentList)> {
    let start = l.end;
    let end = r.start;
    // dict-grammar.lisp:731-746 (def-generic-synergy expansion)
    if start != end {
        return vec![];
    }
    let test_left = filter_in_seq_set(vec![1005460]);
    let left: Vec<_> = l.segments.iter().filter(|s| test_left(s)).cloned().collect();
    let right: Vec<_> = r.segments.iter().filter(|s| right_filter(s)).cloned().collect();
    if left.is_empty() || right.is_empty() {
        return vec![];
    }
    let syn = Synergy {
        description: Some("shika+neg".to_string()),
        connector: Some(" ".to_string()),
        score: 50,
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
    use crate::dict::conj_prop_dao::ConjProp;
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

    fn prop(neg: Option<bool>) -> ConjProp {
        ConjProp {
            id: 0,
            conj_id: 0,
            pos: "v5k".into(),
            conj_type: 1,
            neg,
            fml: None,
        }
    }

    fn cdata(neg: Option<bool>) -> ConjData {
        ConjData {
            seq: Some(1),
            from: Some(2),
            via: None,
            prop: Some(prop(neg)),
            src_map: vec![],
        }
    }

    fn seg(
        start: usize,
        end: usize,
        seq_set: Vec<i32>,
        conj: Vec<ConjData>,
    ) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: vec![],
                seq_set,
                conj,
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

    fn sl(start: usize, end: usize, segments: Vec<Segment>) -> SegmentList {
        SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }
    }

    // REPL probes:
    // - /tmp/probe_442_448.lisp on .103, 2026-05-18: t / nil cases.
    // - /tmp/probe_shika_null.lisp on .103: :NULL case (DB-null neg
    //   keyword is truthy in CL, so synergy fires).
    //
    // Rust ↔ Lisp neg mapping (parse_opt_bool, audit/common/mod.rs:1789):
    //   Some(true)  ↔ Lisp t      → FIRE
    //   Some(false) ↔ Lisp nil    → reject
    //   None        ↔ Lisp :NULL  → FIRE (:NULL is a truthy keyword)

    #[test]
    fn positive_neg_t() {
        // shika-negative/positive (neg=t): RIGHT-SL start=2 end=5 segs=1,
        // SYN desc="shika+neg" conn=" " score=50 start=2 end=2,
        // LEFT-SL start=0 end=2 segs=1.
        let l = sl(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = sl(2, 5, vec![seg(2, 5, vec![], vec![cdata(Some(true))])]);
        let got = synergy_shika_negative(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 5);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("shika+neg"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 50);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_neg_null() {
        // shika-negative/neg=:NULL ALONE -- expect FIRE (REPL
        // /tmp/probe_shika_null.lisp: COUNT=1 desc="shika+neg"
        // score=50). :NULL keyword is truthy in CL, so `some` returns
        // truthy and the filter accepts.
        let l = sl(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = sl(2, 5, vec![seg(2, 5, vec![], vec![cdata(None)])]);
        let got = synergy_shika_negative(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.description.as_deref(), Some("shika+neg"));
        assert_eq!(got[0].1.score, 50);
    }

    #[test]
    fn right_neg_nil_empty() {
        // shika-negative/neg=NIL ALONE -- expect NIL (REPL
        // /tmp/probe_shika_null.lisp). Lisp nil is the sole falsy
        // value, so `some` returns nil and the filter rejects.
        let l = sl(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = sl(2, 5, vec![seg(2, 5, vec![], vec![cdata(Some(false))])]);
        assert!(synergy_shika_negative(&l, &r).is_empty());
    }

    #[test]
    fn right_empty_conj_empty() {
        // shika-negative/right-empty-conj: NIL.
        let l = sl(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = sl(2, 5, vec![seg(2, 5, vec![], vec![])]);
        assert!(synergy_shika_negative(&l, &r).is_empty());
    }

    #[test]
    fn left_miss_empty() {
        // shika-negative/left-miss: NIL.
        let l = sl(0, 2, vec![seg(0, 2, vec![99], vec![])]);
        let r = sl(2, 5, vec![seg(2, 5, vec![], vec![cdata(Some(true))])]);
        assert!(synergy_shika_negative(&l, &r).is_empty());
    }

    #[test]
    fn not_adjacent_empty() {
        // shika-negative/not-adjacent: NIL.
        let l = sl(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = sl(4, 7, vec![seg(4, 7, vec![], vec![cdata(Some(true))])]);
        assert!(synergy_shika_negative(&l, &r).is_empty());
    }

    #[test]
    fn multi_conj_nil_plus_nil_empty() {
        // shika-negative/neg=NIL+NIL -- expect NIL (REPL probe).
        let l = sl(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = sl(
            2,
            5,
            vec![seg(2, 5, vec![], vec![cdata(Some(false)), cdata(Some(false))])],
        );
        assert!(synergy_shika_negative(&l, &r).is_empty());
    }

    #[test]
    fn multi_conj_nil_plus_null_fires() {
        // shika-negative/neg=NIL+:NULL -- expect FIRE (REPL probe).
        // The :NULL cdata's truthy neg-value satisfies `some`.
        let l = sl(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = sl(
            2,
            5,
            vec![seg(2, 5, vec![], vec![cdata(Some(false)), cdata(None)])],
        );
        let got = synergy_shika_negative(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 50);
    }

    #[test]
    fn multi_conj_nil_plus_t_fires() {
        // shika-negative/multi-conj-mixed: RIGHT-SL segs=1, LEFT-SL segs=1.
        // Mirrors the original probe_442_448 test but with the
        // corrected nil mapping (Some(false), not None).
        let l = sl(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = sl(
            2,
            5,
            vec![seg(
                2,
                5,
                vec![],
                vec![cdata(Some(false)), cdata(Some(true))],
            )],
        );
        let got = synergy_shika_negative(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, _syn, left_sl) = &got[0];
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(left_sl.segments.len(), 1);
    }
}
