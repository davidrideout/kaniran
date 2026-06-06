//! Port of `ichiran/dict:def-generic-synergy` (`dict-grammar.lisp:731-746`).
//!
//! Shared body for the generic synergy definers: when the two adjacent
//! segment-lists abut and each has segments passing its left/right
//! filter, emit a `(right-list, synergy, left-list)` triple over the
//! filtered segments.

use std::sync::Arc;

use super::kani_lite_segment::KaniLiteSegment;
use super::kani_lite_segment_list::{make_kani_lite_segment_list_from, KaniLiteSegmentList};
use super::synergy_struct::Synergy;

pub struct DefGenericSynergyOpts<'a> {
    pub description: Option<&'a str>,
    pub connector: &'a str,
    pub score: i32,
}

pub fn def_generic_synergy_body(
    segment_list_left: &KaniLiteSegmentList,
    segment_list_right: &KaniLiteSegmentList,
    filter_left: impl Fn(&Arc<KaniLiteSegment>) -> bool,
    filter_right: impl Fn(&Arc<KaniLiteSegment>) -> bool,
    opts: &DefGenericSynergyOpts<'_>,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    let start = segment_list_left.end;
    let end = segment_list_right.start;
    // dict-grammar.lisp:737 (when (= start end))
    if start != end {
        return vec![];
    }
    // dict-grammar.lisp:738-739 (remove-if-not filter-left/right over segment-list-segments)
    let left: Vec<Arc<KaniLiteSegment>> = segment_list_left
        .segments
        .iter()
        .filter(|s| filter_left(s))
        .cloned()
        .collect();
    let right: Vec<Arc<KaniLiteSegment>> = segment_list_right
        .segments
        .iter()
        .filter(|s| filter_right(s))
        .cloned()
        .collect();
    // dict-grammar.lisp:740 (when (and left right))
    if left.is_empty() || right.is_empty() {
        return vec![];
    }
    // dict-grammar.lisp:741-746 (list (list (make-segment-list-from r right) (make-synergy ...) (make-segment-list-from l left)))
    let syn = Synergy {
        description: opts.description.map(|d| d.to_string()),
        connector: Some(opts.connector.to_string()),
        score: opts.score,
        start,
        end,
    };
    vec![(
        Arc::new(make_kani_lite_segment_list_from(segment_list_right, right)),
        syn,
        Arc::new(make_kani_lite_segment_list_from(segment_list_left, left)),
    )]
}
