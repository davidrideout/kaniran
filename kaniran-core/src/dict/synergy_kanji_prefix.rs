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
//! - The `filter-is-pos` macro expansion (`dict-grammar.lisp:757-764`)
//!   is inlined as a closure on `Segment`'s `kpcl` tuple and `posi`
//!   list per CONVENTIONS §4.6. Here the `kpcl-test` body is `k`
//!   alone — only the kanji-or-katakana slot is consulted.
//! - `pushnew ',name *synergy-list*` from the `defsynergy` expansion
//!   moves to the `*synergy-list*` port (separate wave).

use super::filter_in_seq_set::filter_in_seq_set;
use super::make_segment_list_from::make_segment_list_from;
use super::segment_list_struct::SegmentList;
use super::segment_struct::Segment;
use super::synergy_struct::Synergy;

pub fn synergy_kanji_prefix(
    l: &SegmentList,
    r: &SegmentList,
) -> Vec<(SegmentList, Synergy, SegmentList)> {
    let start = l.end;
    let end = r.start;
    // dict-grammar.lisp:731-746 (def-generic-synergy expansion)
    if start != end {
        return vec![];
    }
    let test_left = filter_in_seq_set(vec![2242840, 1922780, 2423740]);
    // dict-grammar.lisp:757-764 (filter-is-pos macro expansion)
    let test_right = |seg: &Segment| -> bool {
        let info = match &seg.info {
            Some(info) => info,
            None => return false,
        };
        let (k, _p, _c, _l) = info.kpcl;
        k && info.posi.iter().any(|x| x == "n")
    };
    let left: Vec<_> = l.segments.iter().filter(|s| test_left(s)).cloned().collect();
    let right: Vec<_> = r.segments.iter().filter(|s| test_right(s)).cloned().collect();
    if left.is_empty() || right.is_empty() {
        return vec![];
    }
    let syn = Synergy {
        description: Some("kanji prefix+noun".to_string()),
        connector: Some(String::new()),
        score: 15,
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

    // REPL probes (/tmp/probe_synergies.lisp on .103, 2026-05-18).

    #[test]
    fn positive_mi() {
        // kanji-prefix/positive-mi: l seq 2242840, r k=T posi=("n").
        // RIGHT-SL start=1 end=3 segs=1, SYNERGY desc="kanji prefix+noun"
        // conn="" score=15 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = sl(0, 1, vec![seg((false, false, false, false), vec![], vec![2242840])]);
        let r = sl(1, 3, vec![seg((true, false, false, false), vec!["n"], vec![])]);
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
        let l = sl(0, 1, vec![seg((false, false, false, false), vec![], vec![1922780])]);
        let r = sl(1, 3, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let got = synergy_kanji_prefix(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 15);
    }

    #[test]
    fn positive_2423740() {
        // kanji-prefix/positive-2423740: l seq 2423740.
        let l = sl(0, 1, vec![seg((false, false, false, false), vec![], vec![2423740])]);
        let r = sl(1, 3, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let got = synergy_kanji_prefix(&l, &r);
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn neg_no_k() {
        // kanji-prefix/neg-no-k: r kpcl k=NIL even with posi=("n") -> NIL.
        let l = sl(0, 1, vec![seg((false, false, false, false), vec![], vec![2242840])]);
        let r = sl(1, 3, vec![seg((false, false, false, true), vec!["n"], vec![])]);
        assert!(synergy_kanji_prefix(&l, &r).is_empty());
    }

    #[test]
    fn neg_no_n_posi() {
        // kanji-prefix/neg-no-n-posi: r k=T but posi=("v5k") (not "n").
        let l = sl(0, 1, vec![seg((false, false, false, false), vec![], vec![2242840])]);
        let r = sl(1, 3, vec![seg((true, false, false, false), vec!["v5k"], vec![])]);
        assert!(synergy_kanji_prefix(&l, &r).is_empty());
    }

    #[test]
    fn neg_left_miss() {
        // kanji-prefix/neg-left-miss: l seq 9999 doesn't match.
        let l = sl(0, 1, vec![seg((false, false, false, false), vec![], vec![9999])]);
        let r = sl(1, 3, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        assert!(synergy_kanji_prefix(&l, &r).is_empty());
    }
}
