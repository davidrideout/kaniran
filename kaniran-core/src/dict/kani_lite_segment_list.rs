//! Kaniran sidecar (no Lisp FQN). The lite mirror of
//! [`super::path::SegmentList`] used inside the
//! `find-best-path` inner loop, carrying [`Arc<KaniLiteSegment>`]s and
//! a per-list lite [`KaniLiteTopArray`].

use std::sync::Arc;

use super::kani_lite_segment::KaniLiteSegment;
use super::kani_lite_top_array::KaniLiteTopArray;
use super::path::SegmentList;
use super::scoring::score::Segment;

#[derive(Debug, Clone)]
pub struct KaniLiteSegmentList {
    pub start: usize,
    pub end: usize,
    pub matches: usize,
    pub segments: Vec<Arc<KaniLiteSegment>>,
    pub top: Option<Arc<KaniLiteTopArray>>,
}

impl KaniLiteSegmentList {
    /// Build a [`KaniLiteSegmentList`] from a source [`SegmentList`].
    /// Each contained [`Segment`] is converted into a
    /// [`KaniLiteSegment`] with precomputed predicate inputs, sharing
    /// the source list's `Arc<Segment>`.
    pub fn from_segment_list(source: &SegmentList) -> Self {
        let segments = source
            .segments
            .iter()
            .map(|s| Arc::new(KaniLiteSegment::from_segment(Arc::clone(s))))
            .collect();
        Self {
            start: source.start,
            end: source.end,
            matches: source.matches,
            segments,
            top: None,
        }
    }

    /// Materialize a full [`SegmentList`] for export at
    /// `find-best-path` exit. Shares each `KaniLiteSegment.source`
    /// by `Arc` — matching upstream, where the exported payloads hold
    /// the same segment objects as the input segment-lists.
    pub fn to_segment_list(&self) -> SegmentList {
        let segments: Vec<Arc<Segment>> =
            self.segments.iter().map(|s| Arc::clone(&s.source)).collect();
        SegmentList {
            segments,
            start: self.start,
            end: self.end,
            top: None,
            matches: self.matches,
        }
    }
}

/// Sidecar mirror of [`super::grammar::synergy::make_segment_list_from`]
/// for the lite layer. Builds a fresh [`KaniLiteSegmentList`] with
/// the given filtered segments, inheriting `start` / `end` /
/// `matches` / `top` from `old`. Matches the upstream
/// `(copy-segment-list source)` + slot-overwrite pattern.
pub fn make_kani_lite_segment_list_from(
    old: &KaniLiteSegmentList,
    segments: Vec<Arc<KaniLiteSegment>>,
) -> KaniLiteSegmentList {
    KaniLiteSegmentList {
        segments,
        start: old.start,
        end: old.end,
        top: old.top.clone(),
        matches: old.matches,
    }
}
