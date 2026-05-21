//! Port of `ichiran/dict:find-best-path` (`dict.lisp:1190`).
//!
//! ```lisp
//! (defun find-best-path (segment-lists str-length &key (limit 5))
//!   "generalized version of old find-best-path that operates on segment-lists and uses synergies"
//!   ...)
//! ```
//!
//! Builds a top-`limit` array of path scorings over the input
//! segment-lists. Each result is a `(path, score)` pair where `path`
//! is a heterogeneous list of [`SegmentList`]s (left-to-right picks)
//! and [`super::synergy_struct::Synergy`]s (inter-slice bonuses
//! produced by `get-penalties` / `get-synergies` via
//! [`get_seg_splits`]). The initial seed
//! `(register-item top (gap-penalty 0 str-length) nil)` ensures the
//! all-gap "no segments picked" candidate is in the pool.
//!
//! Divergences from Lisp:
//! - The inner loop runs on [`KaniLiteSegmentList`] +
//!   [`KaniLitePathElement`] sidecar types — every predicate input is
//!   precomputed at conversion. Surviving top-K paths are
//!   reconstructed into full [`PathElement`]s at exit.
//! - The Lisp slot `(segment-list-top seg)` is held in a parallel
//!   `Vec<KaniLiteTopArray>` indexed by the same `i` as the lite
//!   list; this avoids juggling [`Arc::make_mut`] across the inner
//!   `(i, j, tai)` loop where two distinct lists' top slots are read
//!   and written in the same iteration.
//! - `limit: Option<usize>` preserves the upstream `&key (limit 5)`
//!   default at the function boundary.

use std::sync::Arc;

use crate::conn::kani_context::KaniranContext;
use crate::dict::expand_segment_list::expand_segment_list;
use crate::dict::gap_penalty::gap_penalty;
use crate::dict::get_seg_initial::get_seg_initial;
use crate::dict::get_seg_splits::get_seg_splits;
use crate::dict::get_segment_score::{get_segment_score, KaniSegmentScoreArg};
use crate::dict::kani_lite_segment_list::KaniLiteSegmentList;
use crate::dict::kani_lite_top_array::{
    kani_lite_get_array, kani_lite_register_item, KaniLiteTopArray,
};
use crate::dict::kani_lite_top_array_item::{KaniLitePathElement, KaniLiteTopArrayItem};
use crate::dict::segment_list_struct::SegmentList;
use crate::dict::top_array_item_struct::PathElement;

const DEFAULT_LIMIT: usize = 5;

pub async fn find_best_path(
    ctx: &KaniranContext,
    segment_lists: &mut [SegmentList],
    str_length: usize,
    limit: Option<usize>,
) -> Result<Vec<(Vec<PathElement>, i32)>, sqlx::Error> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT);

    // dict.lisp:1195-1196 — expand-segment-list mutates each input
    // SegmentList. Do this on the FULL list before lite conversion so
    // the slot mutation is preserved upstream.
    for sl in segment_lists.iter_mut() {
        expand_segment_list(ctx, sl).await?;
    }

    // Build lite sidecars; the per-list top-arrays live in a parallel
    // Vec to keep mutation simple in the inner loop.
    let lite_lists: Vec<Arc<KaniLiteSegmentList>> = segment_lists
        .iter()
        .map(|sl| Arc::new(KaniLiteSegmentList::from_segment_list(sl)))
        .collect();
    let mut per_list_tops: Vec<KaniLiteTopArray> =
        (0..lite_lists.len()).map(|_| KaniLiteTopArray::new(limit)).collect();

    // dict.lisp:1192 (let ((top (make-instance 'top-array :limit limit))))
    let mut top = KaniLiteTopArray::new(limit);

    // dict.lisp:1193 (register-item top (gap-penalty 0 str-length) nil)
    kani_lite_register_item(
        &mut top,
        gap_penalty(0, str_length) as i32,
        Arc::<[KaniLitePathElement]>::from(Vec::new()),
    );

    let n = lite_lists.len();
    // dict.lisp:1200 (loop for (seg1 . rest) on segment-lists ...)
    for i in 0..n {
        let seg1 = Arc::clone(&lite_lists[i]);
        let seg1_start = seg1.start;
        let seg1_end = seg1.end;

        // dict.lisp:1202-1203
        let gap_left_outer = gap_penalty(0, seg1_start);
        let gap_right_outer = gap_penalty(seg1_end, str_length);

        // dict.lisp:1204 (let ((initial-segs (get-seg-initial seg1))))
        let initial_segs = get_seg_initial(&seg1);

        // dict.lisp:1205-1209 (loop for seg in initial-segs ...)
        for seg in initial_segs {
            // dict.lisp:1206 (for score1 = (get-segment-score seg))
            let score1 =
                get_segment_score(&KaniSegmentScoreArg::KaniLiteSegmentList(&seg))
                    .expect("get-seg-initial output carries a scored first segment");
            let payload: Arc<[KaniLitePathElement]> = Arc::from(vec![
                KaniLitePathElement::SegmentList(Arc::clone(&seg)),
            ]);
            // dict.lisp:1208 (register-item (segment-list-top seg1) (+ gap-left score1) (list seg))
            kani_lite_register_item(
                &mut per_list_tops[i],
                (gap_left_outer + score1 as i64) as i32,
                Arc::clone(&payload),
            );
            // dict.lisp:1209 (register-item top (+ gap-left score1 gap-right) (list seg))
            kani_lite_register_item(
                &mut top,
                (gap_left_outer + score1 as i64 + gap_right_outer) as i32,
                payload,
            );
        }

        // dict.lisp:1210-1227 (loop for seg2 in rest ...)
        for j in (i + 1)..n {
            let seg2 = Arc::clone(&lite_lists[j]);
            let seg2_start = seg2.start;
            let seg2_end = seg2.end;

            if seg2_start < seg1_end {
                continue;
            }

            let score2 =
                get_segment_score(&KaniSegmentScoreArg::KaniLiteSegmentList(&seg2))
                    .expect("post-expand segment-list carries a scored first segment");

            let gap_left = gap_penalty(seg1_end, seg2_start);
            let gap_right = gap_penalty(seg2_end, str_length);

            // dict.lisp:1215 — snapshot seg1.top entries before
            // mutating seg2.top in the inner loop.
            let tais: Vec<KaniLiteTopArrayItem> = kani_lite_get_array(&per_list_tops[i])
                .iter()
                .filter_map(|slot| slot.clone())
                .collect();

            for tai in tais {
                // dict.lisp:1216 (for (seg-left . tail) = (tai-payload tai))
                let payload_slice: &[KaniLitePathElement] = &tai.payload;
                if payload_slice.is_empty() {
                    panic!(
                        "tai-payload must be non-empty (per-list top entries via dict.lisp:1208 / :1226)"
                    );
                }
                let seg_left_sl = match &payload_slice[0] {
                    KaniLitePathElement::SegmentList(sl) => Arc::clone(sl),
                    KaniLitePathElement::Synergy(_) => {
                        panic!("tai-payload head is always a SegmentList")
                    }
                };
                let tail: &[KaniLitePathElement] = &payload_slice[1..];

                let score3 = get_segment_score(&KaniSegmentScoreArg::KaniLiteSegmentList(
                    &seg_left_sl,
                ))
                .expect("payload-head segment-list is scored");
                let score_tail = tai.score - score3;

                let splits = get_seg_splits(&seg_left_sl, &seg2);
                for split in splits {
                    let split_sum: i32 = split
                        .iter()
                        .map(|elem| {
                            let arg = match elem {
                                KaniLitePathElement::SegmentList(sl) => {
                                    KaniSegmentScoreArg::KaniLiteSegmentList(sl)
                                }
                                KaniLitePathElement::Synergy(s) => {
                                    KaniSegmentScoreArg::Synergy(s)
                                }
                            };
                            get_segment_score(&arg)
                                .expect("split element is scored (get-seg-splits output)")
                        })
                        .sum();
                    let max_score = split_sum.max(score3 + 1).max(score2 + 1);
                    let accum_i64 = gap_left + max_score as i64 + score_tail as i64;
                    let accum = accum_i64 as i32;

                    // dict.lisp:1225 (for path = (nconc split tail))
                    let mut path_vec: Vec<KaniLitePathElement> = split;
                    path_vec.extend_from_slice(tail);
                    let path: Arc<[KaniLitePathElement]> = Arc::from(path_vec);

                    // dict.lisp:1226 (register-item (segment-list-top seg2) accum path)
                    kani_lite_register_item(&mut per_list_tops[j], accum, Arc::clone(&path));
                    // dict.lisp:1227 (register-item top (+ accum gap-right) path)
                    kani_lite_register_item(&mut top, (accum_i64 + gap_right) as i32, path);
                }
            }
        }
    }

    // dict.lisp:1232-1233 — collect surviving top-K paths and
    // reconstruct full PathElements via deep-clone of each
    // KaniLiteSegment.source.
    let mut result = Vec::new();
    for slot in kani_lite_get_array(&top) {
        let tai = slot
            .as_ref()
            .expect("get-array prefix slots are always Some (register-item invariant)");
        let mut full_path: Vec<PathElement> = tai
            .payload
            .iter()
            .map(|elem| match elem {
                KaniLitePathElement::SegmentList(lite) => {
                    PathElement::SegmentList(lite.to_segment_list())
                }
                KaniLitePathElement::Synergy(s) => PathElement::Synergy(s.clone()),
            })
            .collect();
        full_path.reverse();
        result.push((full_path, tai.score));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    //! Empty-input unit tests pinned against `.103` REPL probes
    //! (SBCL 2.2.9, 2026-05-19). The DB-dependent non-empty paths
    //! (outer loop, get-seg-initial, get-seg-splits accumulation) are
    //! covered by the 522K-row audit binary at
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
