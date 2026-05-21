//! Port of `ichiran/dict:filter-is-noun` (`dict-grammar.lisp:748`).
//!
//! ```lisp
//! (defun filter-is-noun (segment)
//!   (or (destructuring-bind (k p c l) (getf (segment-info segment) :kpcl)
//!         (and (or l k (and p c))
//!              (intersection '("n" "n-adv" "n-t" "adj-na" "n-suf" "pn")
//!                            (getf (segment-info segment) :posi)
//!                            :test 'equal)))
//!       (and (typep (segment-word segment) 'counter-text)
//!            (getf (segment-info segment) :seq-set))))
//! ```
//!
//! Both branches map directly to lite-precomputed fields:
//! - the kpcl-gated branch reads [`KPCL_K`] / [`KPCL_L`] / [`KPCL_P`]
//!   / [`KPCL_C`] bits and intersects against [`POS_NOUN`] (precomputed
//!   union of the 6 noun POS tags).
//! - the counter branch reads [`KaniLiteSegment::is_counter`] +
//!   `!seq_set.is_empty()`.

use std::sync::Arc;

use super::kani_lite_segment::{KaniLiteSegment, KPCL_C, KPCL_K, KPCL_L, KPCL_P, POS_NOUN};

pub fn filter_is_noun(segment: &Arc<KaniLiteSegment>) -> bool {
    let kpcl = segment.kpcl;
    let kpcl_gate =
        (kpcl & (KPCL_L | KPCL_K)) != 0 || (kpcl & KPCL_P != 0 && kpcl & KPCL_C != 0);
    if kpcl_gate && (segment.pos & POS_NOUN) != 0 {
        return true;
    }
    segment.is_counter && !segment.seq_set.is_empty()
}
