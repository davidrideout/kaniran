//! Kaniran sidecar (no Lisp FQN). Lite mirror of
//! [`super::top_array_item_struct::TopArrayItem`] used inside the
//! `find-best-path` inner loop. Payload is [`Arc<[_]>`] so the
//! `(register-item (segment-list-top seg2) accum path) /
//! (register-item top (+ accum gap-right) path)` pair in
//! `dict.lisp:1226-1227` becomes a refcount bump instead of a Vec
//! clone (the "Option C" fold-in from `find_best_path.md` session-2).

use std::sync::Arc;

use super::kani_lite_segment_list::KaniLiteSegmentList;
use super::synergy_struct::Synergy;

#[derive(Debug, Clone)]
pub struct KaniLiteTopArrayItem {
    pub score: i32,
    pub payload: Arc<[KaniLitePathElement]>,
}

/// Lite mirror of [`super::top_array_item_struct::PathElement`]. The
/// inner loop builds these; `find-best-path` reconstructs full
/// [`PathElement`]s from them at exit.
///
/// [`PathElement`]: super::top_array_item_struct::PathElement
#[derive(Debug, Clone)]
pub enum KaniLitePathElement {
    SegmentList(Arc<KaniLiteSegmentList>),
    Synergy(Synergy),
}
