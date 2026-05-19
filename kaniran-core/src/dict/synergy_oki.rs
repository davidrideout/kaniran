//! Port of `ichiran/dict:synergy-oki` (`dict-grammar.lisp:951`).
//!
//! ```lisp
//! (def-generic-synergy synergy-oki (l r)
//!   (filter-is-pos ("ctr") (segment k p c l) t)
//!   (filter-in-seq-set 2854117 2084550)
//!   :score 20
//!   :connector "")
//! ```
//!
//! Divergences from Lisp:
//! - The `filter-is-pos` macro expansion (`dict-grammar.lisp:757-764`)
//!   is inlined as a closure on `Segment`'s `kpcl` tuple and `posi`
//!   list per CONVENTIONS §4.6. The `kpcl-test` body is `t`
//!   (unconditional), so the closure only checks the posi membership.
//! - The macro call omits `:description`, so the slot is `nil` —
//!   mapped to [`None`] per [`Synergy::description`]
//!   (`synergy-struct` doc-comment notes some synergies leave it nil).
//! - `pushnew ',name *synergy-list*` from the `defsynergy` expansion
//!   moves to the `*synergy-list*` port (separate wave).
//!
//! [`Synergy::description`]: super::synergy_struct::Synergy#structfield.description

use super::filter_in_seq_set::filter_in_seq_set;
use super::make_segment_list_from::make_segment_list_from;
use super::segment_list_struct::SegmentList;
use super::segment_struct::Segment;
use super::synergy_struct::Synergy;

pub fn synergy_oki(
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
        info.posi.iter().any(|x| x == "ctr")
    };
    let test_right = filter_in_seq_set(vec![2854117, 2084550]);
    let left: Vec<_> = l.segments.iter().filter(|s| test_left(s)).cloned().collect();
    let right: Vec<_> = r.segments.iter().filter(|s| test_right(s)).cloned().collect();
    if left.is_empty() || right.is_empty() {
        return vec![];
    }
    let syn = Synergy {
        description: None,
        connector: Some(String::new()),
        score: 20,
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
    fn positive_2854117() {
        // oki/positive-2854117: l posi=("ctr") kpcl all nil, r seq 2854117.
        // RIGHT-SL start=1 end=3 segs=1, SYNERGY desc=NIL conn=""
        // score=20 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = sl(0, 1, vec![seg((false, false, false, false), vec!["ctr"], vec![])]);
        let r = sl(1, 3, vec![seg((false, false, false, false), vec![], vec![2854117])]);
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
        let l = sl(0, 1, vec![seg((false, false, false, false), vec!["ctr"], vec![])]);
        let r = sl(1, 3, vec![seg((false, false, false, false), vec![], vec![2084550])]);
        let got = synergy_oki(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 20);
    }

    #[test]
    fn neg_no_ctr_posi() {
        // oki/neg-no-ctr-posi: l posi=("n"), not "ctr" — NIL.
        let l = sl(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = sl(1, 3, vec![seg((false, false, false, false), vec![], vec![2854117])]);
        assert!(synergy_oki(&l, &r).is_empty());
    }

    #[test]
    fn neg_right_miss() {
        // oki/neg-right-miss: r seq doesn't match either — NIL.
        let l = sl(0, 1, vec![seg((true, false, false, false), vec!["ctr"], vec![])]);
        let r = sl(1, 3, vec![seg((false, false, false, false), vec![], vec![9999999])]);
        assert!(synergy_oki(&l, &r).is_empty());
    }

    #[test]
    fn neg_not_adjacent() {
        // oki/neg-not-adjacent: l.end != r.start — NIL.
        let l = sl(0, 1, vec![seg((false, false, false, false), vec!["ctr"], vec![])]);
        let r = sl(5, 7, vec![seg((false, false, false, false), vec![], vec![2854117])]);
        assert!(synergy_oki(&l, &r).is_empty());
    }
}
