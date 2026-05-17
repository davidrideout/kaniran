//! Port of `ichiran/dict:classify` (`dict-grammar.lisp:1032`).
//!
//! Partitions a list into elements that satisfy `filter` and elements
//! that do not, preserving the original order in each output. Upstream:
//!
//! ```lisp
//! (defun classify (filter list)
//!   (loop for element in list
//!      if (funcall filter element)
//!      collect element into yep
//!      else collect element into nope
//!      finally (return (values yep nope))))
//! ```
//!
//! Used by the `def-segfilter-must-follow` macro expansions in
//! `dict-grammar.lisp:1039-1069` to split a [`SegmentList`]'s
//! `segments` field into "satisfies / contradicts" halves before
//! deciding how to recombine them.
//!
//! Divergences from Lisp:
//! - Multi-value return `(values yep nope)` collapses to the Rust
//!   tuple `(Vec<T>, Vec<T>)`, per CONVENTIONS §4 — Lisp multi-value
//!   returns map to Rust tuples by default.
//! - Generic over the element type rather than dynamically typed; in
//!   the upstream codebase every callsite operates on
//!   [`super::segment_struct::Segment`], but the function itself is
//!   untyped and the Rust port preserves that by parameterizing over
//!   `T`.

pub fn classify<T, F>(filter: F, list: &[T]) -> (Vec<T>, Vec<T>)
where
    T: Clone,
    F: Fn(&T) -> bool,
{
    let mut yep: Vec<T> = Vec::new();
    let mut nope: Vec<T> = Vec::new();
    for element in list {
        if filter(element) {
            yep.push(element.clone());
        } else {
            nope.push(element.clone());
        }
    }
    (yep, nope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partitions_by_predicate_preserving_order() {
        // REPL: (classify #'oddp '(1 2 3 4 5)) => yep=(1 3 5) nope=(2 4)
        let (yep, nope) = classify(|n: &i32| n % 2 != 0, &[1, 2, 3, 4, 5]);
        assert_eq!(yep, vec![1, 3, 5]);
        assert_eq!(nope, vec![2, 4]);
    }

    #[test]
    fn empty_input_yields_empty_outputs() {
        // REPL: (classify #'oddp '()) => yep=NIL nope=NIL
        let (yep, nope) = classify(|n: &i32| n % 2 != 0, &[]);
        assert!(yep.is_empty());
        assert!(nope.is_empty());
    }

    #[test]
    fn all_nope_branch() {
        // REPL: (classify #'oddp '(2 4 6)) => yep=NIL nope=(2 4 6)
        let (yep, nope) = classify(|n: &i32| n % 2 != 0, &[2, 4, 6]);
        assert!(yep.is_empty());
        assert_eq!(nope, vec![2, 4, 6]);
    }

    #[test]
    fn all_yep_branch() {
        // REPL: (classify (constantly t) '(1 2 3)) => yep=(1 2 3) nope=NIL
        let (yep, nope) = classify(|_n: &i32| true, &[1, 2, 3]);
        assert_eq!(yep, vec![1, 2, 3]);
        assert!(nope.is_empty());
    }
}
