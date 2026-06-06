//! Port of `ichiran/dict:filter-in-seq-set-simple` (`dict-grammar.lisp:772`).
//!
//! Returns a predicate testing whether a segment's word is non-compound
//! (a single seq) AND its `:seq-set` intersects the supplied list.

use std::sync::Arc;

use super::kani_lite_segment::KaniLiteSegment;

pub fn filter_in_seq_set_simple(seqs: Vec<i32>) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool {
        segment.has_simple_seq && seqs.iter().any(|s| segment.seq_set.contains(s))
    }
}
