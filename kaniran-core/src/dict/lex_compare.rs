//! Port of `ichiran/dict:lex-compare` (`dict-load.lisp:367`).
//!
//! Returns a lexicographic comparator (a closure) parameterised on the
//! element-level `predicate`. Walks two equal-length sequences in
//! lockstep; the first pair where `predicate(e1, e2)` is true makes the
//! comparator return `true`, the first pair where `predicate(e2, e1)`
//! is true makes it return `false`. If neither holds for any pair, the
//! comparator returns `false`. Mismatched lengths walk only the shared
//! prefix and then return `false`.

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
