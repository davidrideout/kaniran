//! Port of `ichiran/dict:segment-list` (`dict.lisp:1038`).
//!
//! Group of [`Segment`]s sharing the same `(start, end)` slice — one
//! per substring that produced at least one above-cutoff segment.

use std::sync::Arc;

use super::segment_struct::Segment;
use super::top_array_class::TopArray;

#[derive(Debug, Clone)]
pub struct SegmentList {
    pub segments: Vec<Segment>,
    pub start: usize,
    pub end: usize,
    /// Divergence from Lisp: wrapped in `Arc` so
    /// `SegmentList::clone()` only bumps a refcount instead of
    /// deep-cloning the accumulator. The Lisp slot is a pointer
    /// (every `(copy-segment-list)` shares the slot value), so the
    /// upstream pointer-share semantics are preserved by `Arc`
    /// better than by `Option<TopArray>` with a derived deep
    /// `Clone`. `find_best_path` is the sole mutator and uses
    /// `Arc::make_mut` on its own per-position slot; downstream
    /// readers only need the shared snapshot.
    pub top: Option<Arc<TopArray>>,
    pub matches: usize,
}
