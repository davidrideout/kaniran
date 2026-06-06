//! Port of `ichiran/dict:top-array-item` (`dict.lisp:1138`).
//!
//! One entry in a [`super::top_array_class::TopArray`] — a
//! `(score, payload)` pair representing one candidate path through the
//! segment graph, where the payload mixes [`SegmentList`] and
//! [`Synergy`] elements.

use super::segment_list_struct::SegmentList;
use super::synergy_struct::Synergy;

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
