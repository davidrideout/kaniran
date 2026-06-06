//! Port of `ichiran/dict:get-digit` (`dict-counters.lisp:94`).
//!
//! Returns the rightmost decimal digit of `n`, or — when that digit
//! is zero — the largest trailing power of ten that divides `n`
//! while its successor in the sequence (10, 100, 1_000, 10_000,
//! 100_000_000) does not. Returns `None` when `n` is divisible by
//! 100_000_000.

pub fn get_digit(n: i64) -> Option<i64> {
    let digit = n % 10;
    if digit != 0 {
        return Some(digit);
    }
    for &(p, pn) in &[
        (10_i64, 100_i64),
        (100, 1_000),
        (1_000, 10_000),
        (10_000, 100_000_000),
    ] {
        if n % pn != 0 {
            return Some(p);
        }
    }
    None
}
