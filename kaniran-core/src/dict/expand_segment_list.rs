//! Port of `ichiran/dict:expand-segment-list` (`dict.lisp:1180`).
//!
//! In-place expansion of a `cull-segments`-shaped [`SegmentList`]:
//! walks `segments` once, asking `get-segsplit` for a compound-text
//! decomposition per row; each non-nil result is appended next to the
//! original and counted against `matches`. The whole list is then
//! stable-sorted high-to-low by `segment-score` and assigned back to
//! the `segments` slot.
//!
//! Diverges from the upstream lambda list `(segment-list)` by taking
//! `&KaniranContext` per CONVENTIONS §4.8 (the descendant
//! [`super::get_segsplit::get_segsplit`] talks to Postgres) and by
//! mutating through `&mut SegmentList` — the Lisp `setf` returns the
//! new segments list, but every caller (`find-best-path`
//! `dict.lisp:1196`) consumes the slot, not the return value, so the
//! Rust port returns `Result<(), sqlx::Error>` instead.

use crate::conn::kani_context::KaniranContext;
use crate::dict::get_segsplit::get_segsplit;
use crate::dict::segment_list_struct::SegmentList;
use crate::dict::segment_struct::Segment;

pub async fn expand_segment_list(
    ctx: &KaniranContext,
    segment_list: &mut SegmentList,
) -> Result<(), sqlx::Error> {
    // dict.lisp:1183-1187 — `(loop for segment in segments for segsplit = (get-segsplit segment) collect segment when segsplit collect segsplit and do (incf matches))`.
    // Move the existing segments out so we can hand each one to
    // get_segsplit by reference, then push owned values into the new
    // working list.
    let pre_segments = std::mem::take(&mut segment_list.segments);
    let mut working: Vec<Segment> = Vec::with_capacity(pre_segments.len() * 2);
    for segment in pre_segments {
        let segsplit = get_segsplit(ctx, &segment).await?;
        working.push(segment);
        if let Some(segsplit) = segsplit {
            working.push(segsplit);
            segment_list.matches += 1;
        }
    }
    // dict.lisp:1188 — `(stable-sort … #'> :key #'segment-score)`. Rust
    // slice `sort_by` is stable; gen-score (`dict.lisp:986`) guarantees
    // every segment reaching this point carries `Some(score)` —
    // `cull-segments` (`dict.lisp:1027`) sorts by `segment-score`
    // upstream and would already have crashed on `nil`.
    working.sort_by(|a, b| {
        let a_score = a
            .score
            .expect("expand-segment-list: segment.score must be Some (cull-segments output)");
        let b_score = b
            .score
            .expect("expand-segment-list: segment.score must be Some (cull-segments output)");
        b_score.cmp(&a_score)
    });
    segment_list.segments = working;
    Ok(())
}
