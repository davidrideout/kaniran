//! Port of `ichiran/dict:filter-is-compound-end-text` (`dict-grammar.lisp:794`).
//!
//! ```lisp
//! (defun filter-is-compound-end-text (&rest texts)
//!   (lambda (segment)
//!     (let* ((word (segment-word segment))
//!            (seq (seq word)))
//!       (and seq (listp (seq word))
//!            (find (get-text (car (last (words word)))) texts :test 'equal)))))
//! ```
//!
//! The compound-and-non-nil gate together with the last child's text
//! collapse to lite-precomputed [`KaniLiteSegment::compound_end_text`]
//! — `Some(text)` exactly when `word` is `Compound`.

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
