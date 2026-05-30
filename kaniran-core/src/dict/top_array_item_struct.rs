//! Port of `ichiran/dict:top-array-item` (`dict.lisp:1138`).
//!
//! One entry in a [`super::top_array_class::TopArray`]'s backing
//! storage — a `(score, payload)` pair representing one candidate
//! path through the segment graph. Built by `register-item`
//! (`dict.lisp:1148-1158`); consumed by `find-best-path`'s outer
//! collect at `dict.lisp:1232-1233` and by the inner score-extension
//! loop at `dict.lisp:1215-1227`.
//!
//! Slot shape (`(defstruct (top-array-item (:conc-name tai-)) score payload)`):
//! - `score` — accumulated path score for this candidate.
//! - `payload` — heterogeneous path-list. Per `slot_types.csv`, every
//!   `register-item` callsite stores a list whose elements are either
//!   [`SegmentList`] entries (the per-slice winner from the inner
//!   loop) or [`Synergy`] records (inter-segment bonuses produced by
//!   `get-penalties` / `get-synergies` via `def-generic-penalty` /
//!   `defsynergy` macro expansion). `fill-segment-path` filters via
//!   `(typep _ 'segment-list)` at `dict.lisp:1398`;
//!   `def-generic-penalty` callsites consume the [`Synergy`] branch.
//!
//! Per CONVENTIONS §4.3 the payload's variant set is closed by a
//! sibling sidecar enum [`PathElement`] in this file.
//!
//! Divergences from Lisp:
//! - `payload` is [`Vec<PathElement>`] rather than a Lisp cons-list
//!   of `t`-typed values. The variant set is type-narrowed per the
//!   `slot_types.csv` rows for `top-array-item.payload`.
//! - `score` is [`i32`] rather than `Option<i32>`; every
//!   `make-top-array-item` callsite passes a score even though the
//!   `defstruct` slot defaults to nil.

use super::segment_list_struct::SegmentList;
use super::grammar::synergy::Synergy;

#[derive(Debug, Clone)]
pub struct TopArrayItem {
    pub score: i32,
    pub payload: Vec<PathElement>,
}

/// Sidecar (no Lisp FQN). Closed variant set for the entries
/// `register-item` stores in [`TopArrayItem::payload`]. Per
/// `slot_types.csv`'s two `top-array-item.payload` rows: the
/// `find-best-path` inner loop (`dict.lisp:1208-1226`) pushes
/// [`SegmentList`] elements; `get-seg-splits` (`dict.lisp:1175-1178`)
/// pushes [`Synergy`] elements via `get-penalties` / `get-synergies`.
#[derive(Debug, Clone)]
pub enum PathElement {
    SegmentList(SegmentList),
    Synergy(Synergy),
}
