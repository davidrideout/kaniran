//! Port of `ichiran/dict:filter-in-seq-set-simple` (`dict-grammar.lisp:772`).
//!
//! Tests whether a segment's `(seq word)` is non-list AND its
//! `:seq-set` intersects the supplied list. Upstream:
//!
//! ```lisp
//! (declaim (inline filter-in-seq-set-simple))
//! (defun filter-in-seq-set-simple (&rest seqs)
//!   "Same as above but checks that the word is not compound"
//!   (lambda (segment)
//!     (let ((seq (seq (segment-word segment))))
//!       (and (not (listp seq))
//!            (intersection seqs (getf (segment-info segment) :seq-set))))))
//! ```
//!
//! Used by `segfilter-n` (`dict-grammar.lisp:1086-1088`).
//!
//! The `(not (listp seq))` gate is precomputed at lite construction
//! as [`KaniLiteSegment::has_simple_seq`] — true iff `seq(word)` is
//! `Single(_)`. Both `WordInfoSeq::Multi` (compound) and `None`
//! (sourceless counter) fail the gate.

use std::sync::Arc;

use super::kani::KaniLiteSegment;

pub fn filter_in_seq_set_simple(seqs: Vec<i32>) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool {
        segment.has_simple_seq && seqs.iter().any(|s| segment.seq_set.contains(s))
    }
}
