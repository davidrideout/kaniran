//! Port of `ichiran/dict:filter-is-compound-end` (`dict-grammar.lisp:786`).
//!
//! Tests whether a segment's word is a compound whose last child's
//! seq matches any of the supplied seqs.

use std::sync::Arc;

use super::kani_lite_segment::KaniLiteSegment;

pub fn filter_is_compound_end(seqs: Vec<i32>) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool {
        match segment.compound_end_seq {
            Some(s) => seqs.contains(&s),
            None => false,
        }
    }
}
