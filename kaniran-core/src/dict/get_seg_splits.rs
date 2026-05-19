//! Port of `ichiran/dict:get-seg-splits` (`dict.lisp:1175`).
//!
//! ```lisp
//! (defun get-seg-splits (seg-left seg-right)
//!   (let ((splits (apply-segfilters seg-left seg-right)))
//!     (loop for (seg-left seg-right) in splits
//!          nconcing (cons (get-penalties seg-left seg-right) (get-synergies seg-left seg-right)))))
//! ```
//!
//! For each `(seg-left, seg-right)` pair produced by
//! [`apply_segfilters`], builds a flat list of paths whose first
//! element is the penalty path from [`get_penalties`] (always present,
//! either the 3-element penalty form or the 2-element fallback) and
//! whose remaining elements are the per-synergy paths from
//! [`get_synergies`].
//!
//! Divergences from Lisp:
//! - `seg_left` is `&SegmentList`, not `Option<&SegmentList>`. Every
//!   `find-best-path` callsite (`dict.lisp:1219`) destructures a
//!   non-empty `tai-payload`, so the input left is always present;
//!   typing it `&SegmentList` lifts the invariant to the borrow
//!   checker. `apply_segfilters` itself still admits `None` for the
//!   `get-seg-initial` callsite (`dict.lisp:1172`).
//! - Each "split" element is `Vec<PathElement>` (CONVENTIONS §4.3) —
//!   the same closed enum [`get_penalties`] and [`get_synergies`] use.
//!
//! [`apply_segfilters`]: super::apply_segfilters::apply_segfilters
//! [`get_penalties`]: super::get_penalties::get_penalties
//! [`get_synergies`]: super::get_synergies::get_synergies

use super::apply_segfilters::apply_segfilters;
use super::get_penalties::get_penalties;
use super::get_synergies::get_synergies;
use super::segment_list_struct::SegmentList;
use super::top_array_item_struct::PathElement;

pub fn get_seg_splits(seg_left: &SegmentList, seg_right: &SegmentList) -> Vec<Vec<PathElement>> {
    // dict.lisp:1176 (let ((splits (apply-segfilters seg-left seg-right))))
    let splits = apply_segfilters(Some(seg_left), seg_right);
    let mut result: Vec<Vec<PathElement>> = Vec::new();
    // dict.lisp:1177-1178 (loop for (seg-left seg-right) in splits
    //                       nconcing (cons (get-penalties seg-left seg-right)
    //                                      (get-synergies seg-left seg-right)))
    for (left_opt, right) in &splits {
        let left = left_opt
            .as_ref()
            .expect("apply_segfilters preserves Some-left when input left is Some");
        result.push(get_penalties(left, right));
        for synergy_path in get_synergies(left, right) {
            result.push(synergy_path);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::conj_prop_dao::ConjProp;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::segment_struct::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::simple_text_class::SimpleText;
    use crate::dict::synergy_struct::Synergy;

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

    fn cdata(conj_type: i32) -> ConjData {
        ConjData {
            seq: None,
            from: None,
            via: None,
            prop: Some(ConjProp {
                id: 0,
                conj_id: 0,
                conj_type,
                pos: String::new(),
                neg: None,
                fml: None,
            }),
            src_map: vec![],
        }
    }

    fn info(
        seq_set: Vec<i32>,
        conj: Vec<ConjData>,
        posi: Vec<&str>,
        kpcl: (bool, bool, bool, bool),
    ) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: posi.into_iter().map(String::from).collect(),
            seq_set,
            conj,
            common: None,
            score_info: KaniScoreInfo {
                prop_score: 0,
                kanji_break: vec![],
                use_length_bonus: 0,
                split_info: KaniSplitInfo::None,
            },
            kpcl,
        }
    }

    fn seg(start: usize, end: usize, info: KaniSegmentInfo, text: &str) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(info),
            top: None,
            text: Some(text.to_string()),
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

    fn unwrap_sl(elem: &PathElement) -> &SegmentList {
        match elem {
            PathElement::SegmentList(sl) => sl,
            other => panic!("expected SegmentList, got {:?}", other),
        }
    }

    fn unwrap_synergy(elem: &PathElement) -> &Synergy {
        match elem {
            PathElement::Synergy(s) => s,
            other => panic!("expected Synergy, got {:?}", other),
        }
    }

    // REPL probes (`/tmp/probe_gss_synth*.lisp` on .103, 2026-05-19).
    // Each case constructs Lisp segment-lists with synthetic kana-text
    // words so apply-segfilters / get-penalties / get-synergies run
    // without DB. The same shapes are reproduced here via synthetic
    // Rust SegmentLists.

    #[test]
    fn a_no_penalty_no_synergy_yields_one_fallback_outer() {
        // probe A: long 3-char spans, kpcl[0]=true (filter-short-kana
        // bails), neutral seq-sets, adjacent. No penalty fires, no
        // synergy fires. apply-segfilters returns one pair → 1 outer
        // with the 2-element fallback `(seg-right, seg-left)`.
        let l = sl(
            0,
            3,
            vec![seg(0, 3, info(vec![9999], vec![], vec![], (true, false, false, false)), "abc")],
        );
        let r = sl(
            3,
            6,
            vec![seg(3, 6, info(vec![8888], vec![], vec![], (true, false, false, false)), "def")],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].len(), 2);
        assert_eq!(unwrap_sl(&got[0][0]).start, 3);
        assert_eq!(unwrap_sl(&got[0][0]).end, 6);
        assert_eq!(unwrap_sl(&got[0][1]).start, 0);
        assert_eq!(unwrap_sl(&got[0][1]).end, 3);
    }

    #[test]
    fn b_penalty_short_only_yields_one_penalty_outer() {
        // probe B: 1-char l, 1-char r, kpcl=(nil,nil,nil,nil), gap=2.
        // penalty-short fires (short kana both sides, :serial nil so
        // gap doesn't matter). No synergy is in scope. 1 outer with
        // the 3-element penalty path `(seg-right, syn-short, seg-left)`.
        let l = sl(
            0,
            1,
            vec![seg(0, 1, info(vec![9999], vec![], vec![], (false, false, false, false)), "あ")],
        );
        let r = sl(
            3,
            4,
            vec![seg(3, 4, info(vec![8888], vec![], vec![], (false, false, false, false)), "い")],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].len(), 3);
        assert_eq!(unwrap_sl(&got[0][0]).start, 3);
        assert_eq!(unwrap_sl(&got[0][0]).end, 4);
        let syn = unwrap_synergy(&got[0][1]);
        assert_eq!(syn.description.as_deref(), Some("short"));
        assert_eq!(syn.score, -9);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 3);
        assert_eq!(unwrap_sl(&got[0][2]).start, 0);
        assert_eq!(unwrap_sl(&got[0][2]).end, 1);
    }

    #[test]
    fn c_synergy_no_adjectives_only_yields_fallback_plus_synergy() {
        // probe C: l kpcl=(t,...), posi="adj-no", 1-char; r seq=1469800
        // (の), 1-char, kpcl=(nil,...), adjacent. penalty-short bails
        // (filter-short-kana rejects kpcl[0]=t left). synergy-no-adjectives
        // fires (filter-is-pos accepts adj-no with k=t). apply-segfilters
        // returns one pair → 1 fallback + 1 synergy = 2 outer.
        let l = sl(
            0,
            1,
            vec![seg(0, 1, info(vec![], vec![], vec!["adj-no"], (true, false, false, false)), "x")],
        );
        let r = sl(
            1,
            2,
            vec![seg(1, 2, info(vec![1469800], vec![], vec![], (false, false, false, false)), "y")],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 2);
        // [0]: fallback 2-elem
        assert_eq!(got[0].len(), 2);
        assert_eq!(unwrap_sl(&got[0][0]).start, 1);
        assert_eq!(unwrap_sl(&got[0][1]).start, 0);
        // [1]: synergy 3-elem
        assert_eq!(got[1].len(), 3);
        let syn = unwrap_synergy(&got[1][1]);
        assert_eq!(syn.description.as_deref(), Some("no-adjective"));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
    }

    #[test]
    fn d_aux_verb_segfilter_split_yields_two_fallback_outers() {
        // probe D: l has 2 segs (conj-type 13, conj-type 3); r has 2
        // segs (seq 1342560 aux-verb, seq 999 non-aux). segfilter-aux-verb
        // splits into 2 pairs (sat-l × sat-r, l × con-r). Neither pair
        // fires a penalty or synergy → 2 outer fallback pairs.
        let l = sl(
            0,
            2,
            vec![
                seg(0, 2, info(vec![], vec![cdata(13)], vec![], (false, false, false, false)), "x1"),
                seg(0, 2, info(vec![], vec![cdata(3)], vec![], (false, false, false, false)), "x2"),
            ],
        );
        let r = sl(
            2,
            4,
            vec![
                seg(2, 4, info(vec![1342560], vec![], vec![], (false, false, false, false)), "y1"),
                seg(2, 4, info(vec![999], vec![], vec![], (false, false, false, false)), "y2"),
            ],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 2);
        for outer in &got {
            assert_eq!(outer.len(), 2);
            assert_eq!(unwrap_sl(&outer[0]).start, 2);
            assert_eq!(unwrap_sl(&outer[1]).start, 0);
        }
    }

    #[test]
    fn e_non_adjacent_blocks_synergy_keeps_fallback() {
        // probe E: l kpcl=(t,...), posi="adj-no", 1-char span [0,1]; r
        // seq=1469800, 1-char span [3,4] — gap=2 → synergy fails (needs
        // adjacency via :serial t). penalty-short also fails (kpcl[0]=t
        // on left rejects filter-short-kana). 1 outer fallback.
        let l = sl(
            0,
            1,
            vec![seg(0, 1, info(vec![], vec![], vec!["adj-no"], (true, false, false, false)), "x")],
        );
        let r = sl(
            3,
            4,
            vec![seg(3, 4, info(vec![1469800], vec![], vec![], (false, false, false, false)), "y")],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].len(), 2);
        assert_eq!(unwrap_sl(&got[0][0]).start, 3);
        assert_eq!(unwrap_sl(&got[0][1]).start, 0);
    }

    #[test]
    fn f_penalty_semi_final_plus_synergy_no_adjectives() {
        // probe F2: l kpcl=(t,...), posi="adj-no", seq-set=(2029110) (な,
        // in *semi-final-prt*); r kpcl=(nil,...), seq=1469800 (の),
        // adjacent. penalty-semi-final fires (first in *penalty-list*,
        // not penalty-short) — synergy-no-adjectives also fires → 1
        // outer penalty 3-elem + 1 outer synergy 3-elem = 2 outer.
        let l = sl(
            0,
            1,
            vec![seg(
                0,
                1,
                info(vec![2029110], vec![], vec!["adj-no"], (true, false, false, false)),
                "x",
            )],
        );
        let r = sl(
            1,
            2,
            vec![seg(1, 2, info(vec![1469800], vec![], vec![], (false, false, false, false)), "y")],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].len(), 3);
        let syn0 = unwrap_synergy(&got[0][1]);
        assert_eq!(syn0.description.as_deref(), Some("semi-final not final"));
        assert_eq!(syn0.score, -15);
        assert_eq!(got[1].len(), 3);
        let syn1 = unwrap_synergy(&got[1][1]);
        assert_eq!(syn1.description.as_deref(), Some("no-adjective"));
        assert_eq!(syn1.score, 15);
    }
}
