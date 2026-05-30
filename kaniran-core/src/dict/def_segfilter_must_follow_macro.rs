//! Port of `ichiran/dict:def-segfilter-must-follow`
//! (`dict-grammar.lisp:1039-1069`).
//!
//! ```lisp
//! (defmacro def-segfilter-must-follow (name (segment-list-left segment-list-right)
//!                                      filter-left filter-right &key allow-first)
//!   …
//!   `(defsegfilter ,name (,segment-list-left ,segment-list-right)
//!      (multiple-value-bind (,satisfies-right ,contradicts-right)
//!          (classify ,filter-right (segment-list-segments ,segment-list-right))
//!        (cond
//!          ((or (not ,satisfies-right) (and ,allow-first (not ,segment-list-left)))
//!           (list (list ,segment-list-left ,segment-list-right)))
//!          ((or (not ,segment-list-left)
//!               (/= (segment-list-end ,segment-list-left) (segment-list-start ,segment-list-right)))
//!           (when ,contradicts-right
//!             (list (list ,segment-list-left
//!                         (make-segment-list-from ,segment-list-right ,contradicts-right)))))
//!          (t
//!           (multiple-value-bind (,satisfies-left ,contradicts-left)
//!               (classify ,filter-left (segment-list-segments ,segment-list-left))
//!             (if ,contradicts-left
//!                 (let ((,result (when ,contradicts-right
//!                                  (list
//!                                   (list ,segment-list-left
//!                                         (make-segment-list-from ,segment-list-right ,contradicts-right))))))
//!                   (when ,satisfies-left
//!                     (push
//!                      (list (make-segment-list-from ,segment-list-left ,satisfies-left)
//!                            (make-segment-list-from ,segment-list-right ,satisfies-right))
//!                      ,result))
//!                   ,result)
//!                 (list (list ,segment-list-left ,segment-list-right))))))))
//! ```
//!
//! Each `(def-segfilter-must-follow …)` callsite in
//! `dict-grammar.lisp` ports to its own `segfilter_*.rs` file that
//! constructs the two filter closures and the `allow_first` flag,
//! then delegates to [`def_segfilter_must_follow_body`].
//!
//! ## Divergences from Lisp
//! - The inner `defsegfilter` macro (`dict-grammar.lisp:1026`) is
//!   absorbed by the static slice
//!   [`super::_star_segfilter_list_star_::SEGFILTER_LIST`]; the
//!   per-callsite Rust function simply has the same name and the
//!   slice references it. This helper emits the function body only,
//!   not the registration.
//! - `seg_left` is `Option<&Arc<KaniLiteSegmentList>>` (Lisp `nil` ⇒
//!   `None`); `seg_right` is always non-nil. Lists threaded through
//!   the segfilter pipeline are `Arc<KaniLiteSegmentList>` so the
//!   pass-through paths reduce to refcount bumps; the Lisp `(list
//!   seg-left seg-right)` is also pointer-shared.

use std::sync::Arc;

use super::classify::classify;
use super::kani::KaniLiteSegment;
use super::kani::{
    make_kani_lite_segment_list_from, KaniLiteSegmentList,
};

pub fn def_segfilter_must_follow_body<FL, FR>(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
    filter_left: FL,
    filter_right: FR,
    allow_first: bool,
) -> Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>
where
    FL: Fn(&Arc<KaniLiteSegment>) -> bool,
    FR: Fn(&Arc<KaniLiteSegment>) -> bool,
{
    let (sat_r, con_r) = classify(filter_right, &seg_right.segments);

    // dict-grammar.lisp:1048-1049 (cond clause 1) — pass through when
    // nothing on the right matches, or when allow-first and l=nil.
    if sat_r.is_empty() || (allow_first && seg_left.is_none()) {
        return vec![(seg_left.cloned(), Arc::clone(seg_right))];
    }

    // dict-grammar.lisp:1050-1054 (cond clause 2) — l absent or
    // non-adjacent: keep only the non-matching right segments.
    let l = match seg_left {
        None => {
            return if con_r.is_empty() {
                Vec::new()
            } else {
                vec![(
                    None,
                    Arc::new(make_kani_lite_segment_list_from(seg_right, con_r)),
                )]
            };
        }
        Some(l) if l.end != seg_right.start => {
            return if con_r.is_empty() {
                Vec::new()
            } else {
                vec![(
                    Some(Arc::clone(l)),
                    Arc::new(make_kani_lite_segment_list_from(seg_right, con_r)),
                )]
            };
        }
        Some(l) => l,
    };

    // dict-grammar.lisp:1055-1069 (t branch) — l adjacent to r:
    // classify l and emit the satisfies × satisfies pair (prepended)
    // alongside the unchanged-l × contradicts-r pair.
    let (sat_l, con_l) = classify(filter_left, &l.segments);

    if con_l.is_empty() {
        return vec![(Some(Arc::clone(l)), Arc::clone(seg_right))];
    }

    let mut result: Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> =
        Vec::new();
    if !con_r.is_empty() {
        result.push((
            Some(Arc::clone(l)),
            Arc::new(make_kani_lite_segment_list_from(seg_right, con_r)),
        ));
    }
    if !sat_l.is_empty() {
        // dict-grammar.lisp:1064 (push) — prepend the satisfies pair.
        result.insert(
            0,
            (
                Some(Arc::new(make_kani_lite_segment_list_from(l, sat_l))),
                Arc::new(make_kani_lite_segment_list_from(seg_right, sat_r)),
            ),
        );
    }
    result
}

#[cfg(test)]
mod tests {
    //! Synthetic-filter tests pinning each branch of the macro
    //! expansion independent of any specific dictionary lookup. The
    //! per-callsite segfilter_*.rs files cover the full pipeline with
    //! real fixtures; these tests give the helper a self-contained
    //! specification.

    use super::*;
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

    fn info(seq_set: Vec<i32>) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: vec![],
            seq_set,
            conj: vec![],
            common: None,
            score_info: KaniScoreInfo {
                prop_score: 0,
                kanji_break: vec![],
                use_length_bonus: 0,
                split_info: KaniSplitInfo::None,
            },
            kpcl: (false, false, false, false),
        }
    }

    fn seg(start: usize, end: usize, seq_set: Vec<i32>) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(info(seq_set)),
            top: None,
            text: None,
        }
    }

    fn sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    /// `sat-r` empty → pass through `(l, r)` unchanged.
    #[test]
    fn clause_1_no_right_match_passes_through() {
        let r = sl(0, 1, vec![seg(0, 1, vec![999])]);
        let result = def_segfilter_must_follow_body(
            None,
            &r,
            |_| true,
            |s| s.seq_set.contains(&100),
            false,
        );
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    /// `allow_first && l=None` → pass through even when sat-r is full.
    #[test]
    fn clause_1_allow_first_passes_through_when_l_none() {
        let r = sl(0, 1, vec![seg(0, 1, vec![100])]);
        let result = def_segfilter_must_follow_body(
            None,
            &r,
            |_| true,
            |s| s.seq_set.contains(&100),
            true,
        );
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    /// `l=None` without `allow_first`, `con_r` empty → empty result.
    #[test]
    fn clause_2_l_none_all_right_matches_returns_empty() {
        let r = sl(0, 1, vec![seg(0, 1, vec![100])]);
        let result = def_segfilter_must_follow_body(
            None,
            &r,
            |_| true,
            |s| s.seq_set.contains(&100),
            false,
        );
        assert!(result.is_empty());
    }

    /// `l=None` without `allow_first`, `con_r` non-empty → drop matching segs.
    #[test]
    fn clause_2_l_none_mixed_right_drops_matches() {
        let r = sl(0, 1, vec![seg(0, 1, vec![100]), seg(0, 1, vec![999])]);
        let result = def_segfilter_must_follow_body(
            None,
            &r,
            |_| true,
            |s| s.seq_set.contains(&100),
            false,
        );
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    /// Gap (l.end ≠ r.start) with `con_r` empty → empty result.
    #[test]
    fn clause_2_gap_all_right_matches_returns_empty() {
        let l = sl(0, 1, vec![seg(0, 1, vec![999])]);
        let r = sl(2, 3, vec![seg(2, 3, vec![100])]);
        let result = def_segfilter_must_follow_body(
            Some(&l),
            &r,
            |_| true,
            |s| s.seq_set.contains(&100),
            true,
        );
        assert!(result.is_empty());
    }

    /// T-branch with `con_l` empty → pass through `(l, r)` unchanged.
    #[test]
    fn t_branch_all_left_satisfies_passes_through() {
        let l = sl(0, 1, vec![seg(0, 1, vec![999])]);
        let r = sl(1, 2, vec![seg(1, 2, vec![100])]);
        let result = def_segfilter_must_follow_body(
            Some(&l),
            &r,
            |_| true,
            |s| s.seq_set.contains(&100),
            false,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    /// T-branch with `sat_l` non-empty and `con_r` non-empty → two pairs,
    /// `(sat_l, sat_r)` prepended.
    #[test]
    fn t_branch_mixed_both_emits_two_pairs() {
        let l = sl(0, 1, vec![seg(0, 1, vec![100]), seg(0, 1, vec![999])]);
        let r = sl(1, 2, vec![seg(1, 2, vec![100]), seg(1, 2, vec![999])]);
        let result = def_segfilter_must_follow_body(
            Some(&l),
            &r,
            |s| s.seq_set.contains(&100),
            |s| s.seq_set.contains(&100),
            false,
        );
        assert_eq!(result.len(), 2);
        // First pair: sat_l × sat_r (prepended).
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![100]);
        assert_eq!(result[0].1.segments[0].seq_set, vec![100]);
        // Second pair: l unchanged × con_r.
        assert_eq!(result[1].0.as_ref().unwrap().segments.len(), 2);
        assert_eq!(result[1].1.segments[0].seq_set, vec![999]);
    }

    /// T-branch with `sat_l` empty and `con_r` non-empty → only the
    /// base pair (no prepended sat-pair).
    #[test]
    fn t_branch_no_left_satisfies_emits_base_only() {
        let l = sl(0, 1, vec![seg(0, 1, vec![999])]);
        let r = sl(1, 2, vec![seg(1, 2, vec![100]), seg(1, 2, vec![999])]);
        let result = def_segfilter_must_follow_body(
            Some(&l),
            &r,
            |_| false,
            |s| s.seq_set.contains(&100),
            false,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    /// T-branch with `sat_l` non-empty and `con_r` empty → only the
    /// prepended sat-pair.
    #[test]
    fn t_branch_no_right_contradicts_emits_sat_only() {
        let l = sl(0, 1, vec![seg(0, 1, vec![100]), seg(0, 1, vec![999])]);
        let r = sl(1, 2, vec![seg(1, 2, vec![100])]);
        let result = def_segfilter_must_follow_body(
            Some(&l),
            &r,
            |s| s.seq_set.contains(&100),
            |s| s.seq_set.contains(&100),
            false,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![100]);
        assert_eq!(result[0].1.segments[0].seq_set, vec![100]);
    }
}
