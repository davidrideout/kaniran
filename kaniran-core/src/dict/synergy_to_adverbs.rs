//! Port of `ichiran/dict:synergy-to-adverbs` (`dict-grammar.lisp:877`).
//!
//! ```lisp
//! (def-generic-synergy synergy-to-adverbs (l r)
//!   (filter-is-pos ("adv-to") (segment k p c l) (or k l p))
//!   (filter-in-seq-set 1008490)
//!   :description "to-adverb"
//!   :score (+ 10 (* 10 (- (segment-list-end l) (segment-list-start l))))
//!   :connector " ")
//! ```
//!
//! Divergences from Lisp:
//! - The `filter-is-pos` macro expansion (`dict-grammar.lisp:757-764`)
//!   is inlined as a closure on `Segment`'s `kpcl` tuple and `posi`
//!   list per CONVENTIONS §4.6. The kpcl-test here is `(or k l p)`
//!   (note: bare `p` without `c`, unlike the sibling `synergy-no-
//!   adjectives` / `synergy-na-adjectives` ports).
//! - `pushnew ',name *synergy-list*` from the `defsynergy` expansion
//!   moves to the `*synergy-list*` port (separate wave).

use super::filter_in_seq_set::filter_in_seq_set;
use super::make_segment_list_from::make_segment_list_from;
use super::segment_list_struct::SegmentList;
use super::segment_struct::Segment;
use super::synergy_struct::Synergy;

pub fn synergy_to_adverbs(
    l: &SegmentList,
    r: &SegmentList,
) -> Vec<(SegmentList, Synergy, SegmentList)> {
    let start = l.end;
    let end = r.start;
    // dict-grammar.lisp:731-746 (def-generic-synergy expansion)
    if start != end {
        return vec![];
    }
    // dict-grammar.lisp:757-764 (filter-is-pos macro expansion)
    let test_left = |seg: &Segment| -> bool {
        let info = match &seg.info {
            Some(info) => info,
            None => return false,
        };
        let (k, p, _c, lv) = info.kpcl;
        (k || lv || p) && info.posi.iter().any(|x| x == "adv-to")
    };
    let test_right = filter_in_seq_set(vec![1008490]);
    let left: Vec<_> = l.segments.iter().filter(|s| test_left(s)).cloned().collect();
    let right: Vec<_> = r.segments.iter().filter(|s| test_right(s)).cloned().collect();
    if left.is_empty() || right.is_empty() {
        return vec![];
    }
    // dict-grammar.lisp:881 (:score (+ 10 (* 10 (- (end l) (start l)))))
    let span = l.end - l.start;
    let score = 10 + 10 * (span as i32);
    let syn = Synergy {
        description: Some("to-adverb".to_string()),
        connector: Some(" ".to_string()),
        score,
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

    // REPL probes (/tmp/probe_449_451.lisp on .103, 2026-05-18).

    #[test]
    fn positive_k_span2() {
        // to-adv/positive-k: l adv-to k=T span=2 -> score = 10 + 10*2 = 30.
        // RIGHT-SL start=2 end=3 segs=1, SYNERGY desc="to-adverb"
        // conn=" " score=30 start=2 end=2, LEFT-SL start=0 end=2 segs=1.
        let l = sl(0, 2, vec![seg((true, false, false, false), vec!["adv-to"], vec![])]);
        let r = sl(2, 3, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        let got = synergy_to_adverbs(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("to-adverb"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 30);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_l_span1() {
        // to-adv/positive-l: l=T span=1 -> score = 20.
        let l = sl(0, 1, vec![seg((false, false, false, true), vec!["adv-to"], vec![])]);
        let r = sl(1, 2, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        let got = synergy_to_adverbs(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 20);
        assert_eq!(got[0].1.start, 1);
        assert_eq!(got[0].1.end, 1);
    }

    #[test]
    fn positive_p_alone_span3() {
        // to-adv/positive-p-alone: p=T c=NIL span=3 -> score = 40. Bare
        // `p` is the divergence vs synergy-no-adjectives / synergy-na-
        // adjectives whose kpcl-test is `(or k l (and p c))`.
        let l = sl(0, 3, vec![seg((false, true, false, false), vec!["adv-to"], vec![])]);
        let r = sl(3, 4, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        let got = synergy_to_adverbs(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 40);
        assert_eq!(got[0].1.start, 3);
        assert_eq!(got[0].1.end, 3);
    }

    #[test]
    fn positive_p_and_c_span4() {
        // to-adv/positive-p-and-c: p=T c=T span=4 -> score = 50.
        let l = sl(0, 4, vec![seg((false, true, true, false), vec!["adv-to"], vec![])]);
        let r = sl(4, 5, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        let got = synergy_to_adverbs(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 50);
        assert_eq!(got[0].1.start, 4);
        assert_eq!(got[0].1.end, 4);
    }

    #[test]
    fn positive_k_span1() {
        // to-adv/positive-span1: k=T span=1 -> score = 20.
        let l = sl(0, 1, vec![seg((true, false, false, false), vec!["adv-to"], vec![])]);
        let r = sl(1, 2, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        let got = synergy_to_adverbs(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 20);
    }

    #[test]
    fn neg_kpcl_all_nil() {
        // to-adv/neg-kpcl-all-nil: NIL.
        let l = sl(0, 2, vec![seg((false, false, false, false), vec!["adv-to"], vec![])]);
        let r = sl(2, 3, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }

    #[test]
    fn neg_c_alone() {
        // to-adv/neg-c-alone: c=T only (no k, no l, no p) — kpcl-test is
        // `(or k l p)` so bare c does not pass.
        let l = sl(0, 2, vec![seg((false, false, true, false), vec!["adv-to"], vec![])]);
        let r = sl(2, 3, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }

    #[test]
    fn neg_wrong_posi() {
        // to-adv/neg-wrong-posi: posi=("n"), not adv-to -> NIL.
        let l = sl(0, 2, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = sl(2, 3, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }

    #[test]
    fn neg_wrong_right_seq() {
        // to-adv/neg-wrong-right: r seq not 1008490.
        let l = sl(0, 2, vec![seg((true, false, false, false), vec!["adv-to"], vec![])]);
        let r = sl(2, 3, vec![seg((false, false, false, false), vec![], vec![9999])]);
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }

    #[test]
    fn neg_non_adjacent() {
        // to-adv/neg-non-adjacent: l.end /= r.start.
        let l = sl(0, 2, vec![seg((true, false, false, false), vec!["adv-to"], vec![])]);
        let r = sl(5, 6, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }

    #[test]
    fn neg_empty_left() {
        // to-adv/neg-empty-left: l segs empty.
        let l = sl(0, 2, vec![]);
        let r = sl(2, 3, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }
}
