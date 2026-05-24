//! Port of `ichiran/dict:conj-type-order` (`dict.lisp:1612`).
//!
//! ```lisp
//! (defun conj-type-order (conj-type)
//!   ;; swaps Continuative and Imperative so that the former is shown first
//!   (case conj-type
//!     (10 13)
//!     (13 10)
//!     (t conj-type)))
//! ```

pub fn conj_type_order(conj_type: i32) -> i32 {
    match conj_type {
        10 => 13,
        13 => 10,
        _ => conj_type,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL fixtures (.103, `ichiran/dict::conj-type-order`), 2026-05-24.
    /// Covers the 10↔13 swap and the identity fall-through.
    #[test]
    fn conj_type_order_fixtures() {
        let cases: &[(i32, i32)] = &[(10, 13), (13, 10), (1, 1), (0, 0), (99, 99)];
        for (conj_type, expected) in cases {
            assert_eq!(conj_type_order(*conj_type), *expected, "conj_type={conj_type}");
        }
    }
}
