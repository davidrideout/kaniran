//! Port of `ichiran/dict:filter-is-conjugation` (`dict-grammar.lisp:780`).
//!
//! Tests whether a segment's `:conj` records include one with the
//! supplied `conj_type`. Upstream:
//!
//! ```lisp
//! (declaim (inline filter-is-conjugation))
//! (defun filter-is-conjugation (conj-type)
//!   (lambda (segment)
//!     (some (lambda (cdata) (eql (conj-type (conj-data-prop cdata)) conj-type))
//!           (getf (segment-info segment) :conj))))
//! ```
//!
//! The set of `conj_type`s is precomputed at lite construction into
//! [`KaniLiteSegment::conj_types`].

use std::sync::Arc;

use super::kani::KaniLiteSegment;

pub fn filter_is_conjugation(conj_type: i32) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool { segment.conj_types.contains(&conj_type) }
}
