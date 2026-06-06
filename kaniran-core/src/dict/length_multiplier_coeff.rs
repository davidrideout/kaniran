//! Port of `ichiran/dict:length-multiplier-coeff` (`dict.lisp:694`).
//!
//! Lookup helper for the `calc-score` length-bonus formula
//! (`dict.lisp:928, 933`). The first argument is a mora count and the
//! second selects one of four pre-tabulated coefficient sequences.
//! Inside the tabled range it returns the coefficient at that index;
//! outside the range it linearly extrapolates from the last tabled
//! value.

use crate::dict::_star_length_coeff_sequences_star_::{
    KaniLengthClass, LENGTH_COEFF_SEQUENCES,
};

pub fn length_multiplier_coeff(length: i64, class: KaniLengthClass) -> i64 {
    // dict.lisp:693 declares `(integer 0 10000)` for the length
    // parameter. Real `assert!` rather than `debug_assert!` so
    // release-profile audit runs catch a negative input loudly
    // instead of silently extrapolating to a negative coefficient.
    assert!(
        length >= 0,
        "length-multiplier-coeff: length must be ≥ 0 (upstream type (integer 0 10000)), got {length}"
    );
    // dict.lisp:696 — (assoc class *length-coeff-sequences*)
    let coeffs: &[i64] = LENGTH_COEFF_SEQUENCES
        .iter()
        .find(|(c, _)| *c == class)
        .map(|(_, c)| *c)
        .expect("class must be in *length-coeff-sequences*");
    // Upstream `(length coeffs)` includes the keyword head; subtract
    // one to get the count of numeric entries, which is exactly
    // `coeffs.len()` here.
    let n = coeffs.len() as i64;
    // dict.lisp:698 — (< 0 length (length coeffs)) i.e. 0 < length < n+1.
    if 0 < length && length <= n {
        // dict.lisp:699 — (elt coeffs length); upstream index 1 maps to
        // Rust index 0 because the keyword head was sliced off.
        coeffs[(length - 1) as usize]
    } else {
        // dict.lisp:700 — (* length (/ (car (last coeffs)) (1- (length coeffs))))
        // Upstream `(/ a b)` produces a CL rational; the `(the
        // (integer 0 1000) …)` cast at the same line asserts the
        // division is exact. All four current rows satisfy that
        // (60/5, 36/6, 24/4, 24/4); flag a future table edit that
        // breaks parity. Real `assert!` so release-profile audit
        // runs surface the table-edit error rather than silently
        // floor-dividing.
        let last = coeffs[(n - 1) as usize];
        assert!(
            last % n == 0,
            "length-multiplier-coeff: (/ {last} {n}) is not exact — \
             *length-coeff-sequences* table edit broke the upstream \
             `(the (integer 0 1000) …)` assertion at dict.lisp:700"
        );
        length * (last / n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // All assertions REPL-pinned against upstream ichiran.
    #[test]
    fn strong_tabled_range() {
        assert_eq!(length_multiplier_coeff(0, KaniLengthClass::Strong), 0);
        assert_eq!(length_multiplier_coeff(1, KaniLengthClass::Strong), 1);
        assert_eq!(length_multiplier_coeff(2, KaniLengthClass::Strong), 8);
        assert_eq!(length_multiplier_coeff(3, KaniLengthClass::Strong), 24);
        assert_eq!(length_multiplier_coeff(4, KaniLengthClass::Strong), 40);
        assert_eq!(length_multiplier_coeff(5, KaniLengthClass::Strong), 60);
    }

    #[test]
    fn strong_extrapolation() {
        // n = 5, last = 60, last/n = 12. length * 12 outside range.
        assert_eq!(length_multiplier_coeff(6, KaniLengthClass::Strong), 72);
        assert_eq!(length_multiplier_coeff(7, KaniLengthClass::Strong), 84);
        assert_eq!(length_multiplier_coeff(8, KaniLengthClass::Strong), 96);
        assert_eq!(length_multiplier_coeff(10, KaniLengthClass::Strong), 120);
        assert_eq!(length_multiplier_coeff(50, KaniLengthClass::Strong), 600);
    }

    #[test]
    fn weak_tabled_range() {
        assert_eq!(length_multiplier_coeff(0, KaniLengthClass::Weak), 0);
        assert_eq!(length_multiplier_coeff(1, KaniLengthClass::Weak), 1);
        assert_eq!(length_multiplier_coeff(2, KaniLengthClass::Weak), 4);
        assert_eq!(length_multiplier_coeff(3, KaniLengthClass::Weak), 9);
        assert_eq!(length_multiplier_coeff(4, KaniLengthClass::Weak), 16);
        assert_eq!(length_multiplier_coeff(5, KaniLengthClass::Weak), 25);
        assert_eq!(length_multiplier_coeff(6, KaniLengthClass::Weak), 36);
    }

    #[test]
    fn weak_extrapolation() {
        // n = 6, last = 36, last/n = 6.
        assert_eq!(length_multiplier_coeff(7, KaniLengthClass::Weak), 42);
        assert_eq!(length_multiplier_coeff(8, KaniLengthClass::Weak), 48);
        assert_eq!(length_multiplier_coeff(100, KaniLengthClass::Weak), 600);
    }

    #[test]
    fn tail_tabled_range() {
        assert_eq!(length_multiplier_coeff(0, KaniLengthClass::Tail), 0);
        assert_eq!(length_multiplier_coeff(1, KaniLengthClass::Tail), 4);
        assert_eq!(length_multiplier_coeff(2, KaniLengthClass::Tail), 9);
        assert_eq!(length_multiplier_coeff(3, KaniLengthClass::Tail), 16);
        assert_eq!(length_multiplier_coeff(4, KaniLengthClass::Tail), 24);
    }

    #[test]
    fn tail_extrapolation() {
        // n = 4, last = 24, last/n = 6.
        assert_eq!(length_multiplier_coeff(5, KaniLengthClass::Tail), 30);
        assert_eq!(length_multiplier_coeff(6, KaniLengthClass::Tail), 36);
        assert_eq!(length_multiplier_coeff(7, KaniLengthClass::Tail), 42);
        assert_eq!(length_multiplier_coeff(8, KaniLengthClass::Tail), 48);
        assert_eq!(length_multiplier_coeff(1000, KaniLengthClass::Tail), 6000);
    }

    #[test]
    fn ltail_tabled_range() {
        assert_eq!(length_multiplier_coeff(0, KaniLengthClass::Ltail), 0);
        assert_eq!(length_multiplier_coeff(1, KaniLengthClass::Ltail), 4);
        assert_eq!(length_multiplier_coeff(2, KaniLengthClass::Ltail), 12);
        assert_eq!(length_multiplier_coeff(3, KaniLengthClass::Ltail), 18);
        assert_eq!(length_multiplier_coeff(4, KaniLengthClass::Ltail), 24);
    }

    #[test]
    fn ltail_extrapolation() {
        // n = 4, last = 24, last/n = 6.
        assert_eq!(length_multiplier_coeff(5, KaniLengthClass::Ltail), 30);
        assert_eq!(length_multiplier_coeff(6, KaniLengthClass::Ltail), 36);
        assert_eq!(length_multiplier_coeff(7, KaniLengthClass::Ltail), 42);
        assert_eq!(length_multiplier_coeff(8, KaniLengthClass::Ltail), 48);
        assert_eq!(length_multiplier_coeff(10000, KaniLengthClass::Ltail), 60000);
    }
}
