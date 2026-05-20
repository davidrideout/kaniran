//! Port of `ichiran/dict:find-best-path` (`dict.lisp:1190`).
//!
//! ```lisp
//! (defun find-best-path (segment-lists str-length &key (limit 5))
//!   "generalized version of old find-best-path that operates on segment-lists and uses synergies"
//!   ...)
//! ```
//!
//! Builds a top-`limit` array of path scorings over the input
//! segment-lists. Each result is a `(path, score)` pair where `path` is
//! a heterogeneous list of [`SegmentList`]s (left-to-right picks) and
//! [`super::synergy_struct::Synergy`]s (inter-slice bonuses produced by
//! `get-penalties` / `get-synergies` via [`get_seg_splits`]). The
//! initial seed `(register-item top (gap-penalty 0 str-length) nil)`
//! ensures the all-gap "no segments picked" candidate is in the pool.
//!
//! Divergences from Lisp:
//! - `segment_lists` is `&mut [SegmentList]` — [`expand_segment_list`]
//!   mutates the `segments` / `matches` slots in place and the per-list
//!   `top` is set then cleared on the same slot.
//! - `limit: Option<usize>` preserves the upstream `&key (limit 5)`
//!   default at the function boundary; `None` accepts the default,
//!   `Some(n)` overrides.

use std::sync::Arc;

use crate::conn::kani_context::KaniranContext;
use crate::dict::expand_segment_list::expand_segment_list;
use crate::dict::gap_penalty::gap_penalty;
use crate::dict::get_array::get_array;
use crate::dict::get_seg_initial::get_seg_initial;
use crate::dict::get_seg_splits::get_seg_splits;
use crate::dict::get_segment_score::{get_segment_score, KaniSegmentScoreArg};
use crate::dict::register_item::register_item;
use crate::dict::segment_list_struct::SegmentList;
use crate::dict::top_array_class::TopArray;
use crate::dict::top_array_item_struct::{PathElement, TopArrayItem};

const DEFAULT_LIMIT: usize = 5;

pub async fn find_best_path(
    ctx: &KaniranContext,
    segment_lists: &mut [SegmentList],
    str_length: usize,
    limit: Option<usize>,
) -> Result<Vec<(Vec<PathElement>, i32)>, sqlx::Error> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT);

    // dict.lisp:1192 (let ((top (make-instance 'top-array :limit limit))))
    let mut top = TopArray::new(limit);

    // dict.lisp:1193 (register-item top (gap-penalty 0 str-length) nil)
    register_item(&mut top, gap_penalty(0, str_length) as i32, vec![]);

    // dict.lisp:1195-1197 (dolist (segment-list segment-lists)
    //                        (expand-segment-list segment-list)
    //                        (setf (segment-list-top segment-list)
    //                              (make-instance 'top-array :limit limit)))
    for sl in segment_lists.iter_mut() {
        expand_segment_list(ctx, sl).await?;
        sl.top = Some(Arc::new(TopArray::new(limit)));
    }

    let n = segment_lists.len();
    // dict.lisp:1200 (loop for (seg1 . rest) on segment-lists ...)
    for i in 0..n {
        let seg1_start = segment_lists[i].start;
        let seg1_end = segment_lists[i].end;

        // dict.lisp:1202-1203 (let ((gap-left (gap-penalty 0 (segment-list-start seg1)))
        //                            (gap-right (gap-penalty (segment-list-end seg1) str-length))))
        let gap_left_outer = gap_penalty(0, seg1_start);
        let gap_right_outer = gap_penalty(seg1_end, str_length);

        // dict.lisp:1204 (let ((initial-segs (get-seg-initial seg1))))
        let initial_segs = get_seg_initial(&segment_lists[i]);

        // dict.lisp:1205-1209 (loop for seg in initial-segs ...)
        for seg in initial_segs {
            // dict.lisp:1206 (for score1 = (get-segment-score seg))
            let score1 = get_segment_score(&KaniSegmentScoreArg::SegmentList(&seg))
                .expect("get-seg-initial output carries a scored first segment");
            let payload = vec![PathElement::SegmentList(seg)];
            // dict.lisp:1208 (register-item (segment-list-top seg1) (+ gap-left score1) (list seg))
            register_item(
                Arc::make_mut(
                    segment_lists[i]
                        .top
                        .as_mut()
                        .expect("seg1.top installed at dict.lisp:1197"),
                ),
                (gap_left_outer + score1 as i64) as i32,
                payload.clone(),
            );
            // dict.lisp:1209 (register-item top (+ gap-left score1 gap-right) (list seg))
            register_item(
                &mut top,
                (gap_left_outer + score1 as i64 + gap_right_outer) as i32,
                payload,
            );
        }

        // dict.lisp:1210-1227 (loop for seg2 in rest ...)
        for j in (i + 1)..n {
            let seg2_start = segment_lists[j].start;
            let seg2_end = segment_lists[j].end;

            // dict.lisp:1212 (when (>= (segment-list-start seg2) (segment-list-end seg1)))
            if seg2_start < seg1_end {
                continue;
            }

            let score2 = get_segment_score(&KaniSegmentScoreArg::SegmentList(&segment_lists[j]))
                .expect("post-expand segment-list carries a scored first segment");

            // dict.lisp:1213-1214 (with gap-left = (gap-penalty (segment-list-end seg1)
            //                                                   (segment-list-start seg2))
            //                       and gap-right = (gap-penalty (segment-list-end seg2)
            //                                                    str-length))
            let gap_left = gap_penalty(seg1_end, seg2_start);
            let gap_right = gap_penalty(seg2_end, str_length);

            // dict.lisp:1215 (for tai across (get-array (segment-list-top seg1))) —
            // clone owned tais so segment_lists[j].top can be mutated below.
            let tais: Vec<TopArrayItem> = {
                let seg1_top = segment_lists[i]
                    .top
                    .as_ref()
                    .expect("seg1.top installed at dict.lisp:1197");
                get_array(seg1_top)
                    .iter()
                    .filter_map(|slot| slot.clone())
                    .collect()
            };

            for tai in tais {
                // dict.lisp:1216 (for (seg-left . tail) = (tai-payload tai))
                let mut payload_iter = tai.payload.into_iter();
                let seg_left = payload_iter.next().expect(
                    "per-list top entries always have a SegmentList head (registered with non-empty payload at dict.lisp:1208 / :1226)",
                );
                let tail: Vec<PathElement> = payload_iter.collect();
                let seg_left_sl = match &seg_left {
                    PathElement::SegmentList(sl) => sl,
                    PathElement::Synergy(_) => {
                        panic!("tai-payload head is always a SegmentList (per-list top entries via dict.lisp:1208 / :1226)")
                    }
                };

                // dict.lisp:1217 (for score3 = (get-segment-score seg-left))
                let score3 = get_segment_score(&KaniSegmentScoreArg::SegmentList(seg_left_sl))
                    .expect("payload-head segment-list is from get-seg-initial / get-seg-splits — scored");
                // dict.lisp:1218 (for score-tail = (- (tai-score tai) score3))
                let score_tail = tai.score - score3;

                // dict.lisp:1219 (for split in (get-seg-splits seg-left seg2))
                let splits = get_seg_splits(seg_left_sl, &segment_lists[j]);
                for split in splits {
                    // dict.lisp:1220-1224 (for accum = (+ gap-left
                    //                                     (max (reduce #'+ split :key #'get-segment-score)
                    //                                          (1+ score3)
                    //                                          (1+ score2))
                    //                                     score-tail))
                    let split_sum: i32 = split
                        .iter()
                        .map(|elem| {
                            let arg = match elem {
                                PathElement::SegmentList(sl) => {
                                    KaniSegmentScoreArg::SegmentList(sl)
                                }
                                PathElement::Synergy(s) => KaniSegmentScoreArg::Synergy(s),
                            };
                            get_segment_score(&arg)
                                .expect("split element is scored (get-seg-splits output)")
                        })
                        .sum();
                    let max_score = split_sum.max(score3 + 1).max(score2 + 1);
                    let accum_i64 = gap_left + max_score as i64 + score_tail as i64;
                    let accum = accum_i64 as i32;

                    // dict.lisp:1225 (for path = (nconc split tail))
                    let mut path = split;
                    path.extend(tail.iter().cloned());

                    // dict.lisp:1226 (register-item (segment-list-top seg2) accum path)
                    register_item(
                        Arc::make_mut(
                            segment_lists[j]
                                .top
                                .as_mut()
                                .expect("seg2.top installed at dict.lisp:1197"),
                        ),
                        accum,
                        path.clone(),
                    );
                    // dict.lisp:1227 (register-item top (+ accum gap-right) path)
                    register_item(&mut top, (accum_i64 + gap_right) as i32, path);
                }
            }
        }
    }

    // dict.lisp:1229-1230 (dolist (segment segment-lists)
    //                        (setf (segment-list-top segment) nil))
    for sl in segment_lists.iter_mut() {
        sl.top = None;
    }

    // dict.lisp:1232-1233 (loop for tai across (get-array top)
    //                       collect (cons (reverse (tai-payload tai)) (tai-score tai)))
    let mut result = Vec::new();
    for slot in get_array(&top) {
        let tai = slot
            .as_ref()
            .expect("get-array prefix slots are always Some (register-item invariant)");
        let mut path = tai.payload.clone();
        path.reverse();
        result.push((path, tai.score));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    //! Empty-input unit tests pinned against `.103` REPL probes
    //! (SBCL 2.2.9, 2026-05-19). The DB-dependent non-empty paths
    //! (outer loop, get-seg-initial, get-seg-splits accumulation) are
    //! covered comprehensively by the 522K-row audit binary at
    //! `audit/dict/find_best_path_test.rs`.

    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    // REPL: (ichiran/dict::find-best-path nil 5) => ((NIL . -2500))
    #[tokio::test]
    async fn empty_input_length_5_default_limit() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 5, None).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty(), "initial gap-seed has empty payload");
        assert_eq!(result[0].1, -2500);
    }

    // REPL: (ichiran/dict::find-best-path nil 0) => ((NIL . 0))
    #[tokio::test]
    async fn empty_input_length_0() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 0, None).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty());
        assert_eq!(result[0].1, 0);
    }

    // REPL: (ichiran/dict::find-best-path nil 1 :limit 3) => ((NIL . -500))
    #[tokio::test]
    async fn empty_input_length_1_limit_3() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 1, Some(3)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty());
        assert_eq!(result[0].1, -500);
    }

    // REPL: (ichiran/dict::find-best-path nil 1 :limit 1) => ((NIL . -500))
    #[tokio::test]
    async fn empty_input_length_1_limit_1() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 1, Some(1)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty());
        assert_eq!(result[0].1, -500);
    }
}
