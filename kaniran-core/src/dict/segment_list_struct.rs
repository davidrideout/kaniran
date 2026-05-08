//! Port of `ichiran/dict:segment-list` (`dict.lisp:1038`).
//!
//! Group of [`Segment`]s sharing the same `(start, end)` slice — one
//! [`SegmentList`] per substring that produced at least one
//! above-cutoff segment in `join-substring-words` (`dict.lisp:1128`).
//! `find-best-path` walks these in order, expanding each via
//! `expand-segment-list` and registering the resulting top scores
//! into the per-list [`TopArray`] under [`SegmentList::top`]
//! (`dict.lisp:1190-1230`).
//!
//! Slot shape (`(defstruct segment-list segments start end (top nil) (matches 0))`):
//! - `segments` — the candidate readings at this slice (per
//!   `slot_types.csv`). Sorted high-to-low by score after
//!   `cull-segments` and `expand-segment-list` run.
//! - `start` / `end` — character offsets of the shared slice in the
//!   source string.
//! - `top` — backing [`TopArray`] for the per-list path scoring; per
//!   `slot_types.csv`. Populated in find-best-path's inner loop and
//!   cleared to [`None`] before the function returns.
//! - `matches` — total number of segments produced for this slice
//!   before culling, plus increments for every `expand-segment-list`
//!   split. `0` initform.
//!
//! Divergences from Lisp:
//! - `start` / `end` are [`usize`] rather than `Option<usize>`. Both
//!   slots have no `defstruct` initform but every `make-segment-list`
//!   callsite passes them; faithful to call-site practice rather
//!   than to the slot's nil default.
//! - `matches` is [`usize`] rather than the Lisp slot's
//!   nil-or-fixnum shape; the slot has `:initform 0` and is
//!   monotone-increment-only (`incf`), so it is always an integer.

use super::segment_struct::Segment;
use super::top_array_class::TopArray;

#[derive(Debug, Clone)]
pub struct SegmentList {
    pub segments: Vec<Segment>,
    pub start: usize,
    pub end: usize,
    pub top: Option<TopArray>,
    pub matches: usize,
}
