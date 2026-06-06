//! Port of `ichiran/dict:filter-is-compound-end-text` (`dict-grammar.lisp:794`).
//!
//! Returns a predicate testing whether a segment's word is a compound
//! whose last child's text matches any of the supplied texts.

use std::sync::Arc;

use super::kani_lite_segment::KaniLiteSegment;

pub fn filter_is_compound_end_text(texts: Vec<String>) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool {
        match segment.compound_end_text.as_deref() {
            Some(end) => texts.iter().any(|t| t == end),
            None => false,
        }
    }
}
