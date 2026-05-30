//! Port of `ichiran/dict:filter-is-compound-end` (`dict-grammar.lisp:786`).
//!
//! Tests whether a segment's word is a compound whose last child's
//! seq matches any of the supplied seqs. Upstream:
//!
//! ```lisp
//! (declaim (inline filter-is-compound-end))
//! (defun filter-is-compound-end (&rest seqs)
//!   (lambda (segment)
//!     (let* ((word (segment-word segment))
//!            (seq (seq word)))
//!       (and seq (listp seq)
//!            (member (car (last seq)) seqs)))))
//! ```
//!
//! Both predicates `seq(word)` non-nil AND `seq` is a list collapse
//! to the lite-precomputed [`KaniLiteSegment::compound_end_seq`] —
//! `Some(_)` exactly when `word` is `Compound` with a `Single(_)` seq
//! on its last child.

use std::sync::Arc;

use super::kani::KaniLiteSegment;

pub fn filter_is_compound_end(seqs: Vec<i32>) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool {
        match segment.compound_end_seq {
            Some(s) => seqs.contains(&s),
            None => false,
        }
    }
}
