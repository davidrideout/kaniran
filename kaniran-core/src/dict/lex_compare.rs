//! Port of `ichiran/dict:lex-compare` (`dict-load.lisp:367`).
//!
//! Returns a lexicographic comparator (a closure) parameterised on the
//! element-level `predicate`. Walks two equal-length sequences in
//! lockstep; the first pair where `predicate(e1, e2)` is true makes the
//! comparator return `true`, the first pair where `predicate(e2, e1)`
//! is true makes it return `false`. If neither holds for any pair
//! (sequences compare equal under `predicate`), the comparator returns
//! `false`. Per the upstream docstring, sequences must be of equal
//! length; mismatched lengths walk only the shared prefix and then
//! return `false`, matching Common Lisp's `(map nil …)` semantics.
//!
//! ## Divergences from Lisp
//!
//! - Returns `impl Fn(&[T], &[T]) -> bool` instead of a CL function
//!   value. Rust closures are statically typed; the generic parameters
//!   `T` (element type) and `P` (predicate type) are inferred from the
//!   `predicate` argument and the eventual call site.
//! - The element predicate takes `&T, &T` rather than CL's by-value
//!   convention, per CONVENTIONS §4.9 (prefer references over clones).
//! - Comparator inputs are slices (`&[T]`) rather than CL `sequence`
//!   designators. Verified against each upstream caller's sort-key
//!   shape:
//!     * `insert-conjugation` (`dict-load.lisp:379`) — `:key #'cdddr`
//!       on `readings`, where the cdddr tail is the propagated
//!       conjugation key (a list of integers). Ports to `&[i32]`.
//!     * `select-conjs-and-props` (`dict.lisp:1645`) — `:key 'third`
//!       returns `(list (if (eql (seq-via conj) :null) 0 1) val)`, a
//!       2-element list of integers. Ports to `&[i32]`.
//!     * `pair-words-by-conj` (`dict-grammar.lisp:64`) — sort key is
//!       a list of `(seq-from via)` 2-tuples, each integers. Ports to
//!       `&[i32]` after flattening the per-conj-id pair into the
//!       outer comparison.
//!   All three are homogeneous integer sequences; the slice signature
//!   is sufficient. If a future caller needs heterogeneous element
//!   types (e.g. a `(i32, &str)` sort key), the slice signature must
//!   be widened or a parallel implementation added — flag at that
//!   port site.

pub fn lex_compare<T, P>(predicate: P) -> impl Fn(&[T], &[T]) -> bool
where
    P: Fn(&T, &T) -> bool,
{
    move |seq1, seq2| {
        // dict-load.lisp:371 (map nil (lambda (e1 e2) …) seq1 seq2)
        for (e1, e2) in seq1.iter().zip(seq2.iter()) {
            if predicate(e1, e2) {
                return true;
            }
            if predicate(e2, e1) {
                return false;
            }
        }
        // dict-load.lisp:370 (block nil …) — falls off `map nil` to nil.
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL: `(funcall (lex-compare #'<) #(1 2 3) #(1 2 4))` → `T`.
    #[test]
    fn smaller_at_last_position_is_less() {
        let cmp = lex_compare(|a: &i32, b: &i32| a < b);
        assert!(cmp(&[1, 2, 3], &[1, 2, 4]));
    }

    /// REPL: `(funcall (lex-compare #'<) #(1 2 4) #(1 2 3))` → `NIL`.
    #[test]
    fn larger_at_last_position_is_not_less() {
        let cmp = lex_compare(|a: &i32, b: &i32| a < b);
        assert!(!cmp(&[1, 2, 4], &[1, 2, 3]));
    }

    /// REPL: `(funcall (lex-compare #'<) #(1 2 3) #(1 2 3))` → `NIL`.
    /// Equal sequences fall off the end of `map nil` to `nil`.
    #[test]
    fn equal_sequences_return_false() {
        let cmp = lex_compare(|a: &i32, b: &i32| a < b);
        assert!(!cmp(&[1, 2, 3], &[1, 2, 3]));
    }

    /// REPL: `(funcall (lex-compare #'<) (list 1 2 3) (list 1 2 4))` → `T`.
    /// List input matches vector input — both go through `map nil`.
    #[test]
    fn list_inputs_behave_the_same() {
        let cmp = lex_compare(|a: &i32, b: &i32| a < b);
        let s1: Vec<i32> = vec![1, 2, 3];
        let s2: Vec<i32> = vec![1, 2, 4];
        assert!(cmp(&s1, &s2));
    }

    /// REPL: `(funcall (lex-compare #'<) #() #())` → `NIL`.
    #[test]
    fn empty_sequences_return_false() {
        let cmp = lex_compare(|a: &i32, b: &i32| a < b);
        let empty: [i32; 0] = [];
        assert!(!cmp(&empty, &empty));
    }

    /// REPL: `(funcall (lex-compare #'<) (list 1 2) (list 1 2 3))` → `NIL`.
    /// Unequal lengths walk the shared prefix only; falls through to
    /// `nil` when the shared prefix compares equal. (Upstream docstring
    /// notes "Only can sort sequences of equal length"; this pins the
    /// observable behavior at the boundary.)
    #[test]
    fn shorter_seq_with_equal_prefix_returns_false() {
        let cmp = lex_compare(|a: &i32, b: &i32| a < b);
        assert!(!cmp(&[1, 2], &[1, 2, 3]));
    }

    /// REPL: `(funcall (lex-compare #'<) (list 1 2 3) (list 1 2))` → `NIL`.
    /// Symmetric mismatched-length case from `lex-compare`'s perspective.
    #[test]
    fn longer_seq_with_equal_prefix_returns_false() {
        let cmp = lex_compare(|a: &i32, b: &i32| a < b);
        assert!(!cmp(&[1, 2, 3], &[1, 2]));
    }

    /// REPL: `(funcall (lex-compare #'<) (list 1) ())` → `NIL`.
    /// Empty-vs-non-empty: `map nil` exits before any pair is examined.
    #[test]
    fn empty_vs_non_empty_returns_false() {
        let cmp = lex_compare(|a: &i32, b: &i32| a < b);
        let empty: [i32; 0] = [];
        assert!(!cmp(&[1], &empty));
    }

    /// First-position dominates: `(2,9) < (3,1)` under `#'<` since the
    /// first pair already settles the order.
    /// REPL: `(funcall (lex-compare #'<) #(2 9) #(3 1))` → `T`.
    #[test]
    fn first_position_dominates_when_unequal() {
        let cmp = lex_compare(|a: &i32, b: &i32| a < b);
        let result = cmp(&[2, 9], &[3, 1]);
        assert!(result);
    }
}
