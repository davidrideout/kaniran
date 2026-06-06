//! Port of `ichiran/dict:filter-is-conjugation` (`dict-grammar.lisp:780`).
//!
//! Tests whether a segment's `:conj` records include one with the
//! supplied `conj_type`.

use std::sync::Arc;

use super::kani_lite_segment::KaniLiteSegment;

pub fn filter_is_conjugation(conj_type: i32) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool { segment.conj_types.contains(&conj_type) }
}
