//! Port of `ichiran/dict:get-digit` (`dict-counters.lisp:94`).
//!
//! Returns the rightmost decimal digit of `n`, or — when that digit
//! is zero — the largest trailing power of ten that divides `n`
//! while its successor in the sequence (10, 100, 1_000, 10_000,
//! 100_000_000) does not. Returns `None` when no successor exists
//! (the Lisp `loop` falls off the end and yields `nil`); this fires
//! when `n` is divisible by 100_000_000.
//!
//! Used by `counter-join` (`dict-counters.lisp:101`) to key per-digit
//! kana modifications: a real digit `1..9` for the units place, or
//! one of the power values `10 / 100 / 1000 / 10000 / 100000000`
//! to mark "tens / hundreds / thousands / ten-thousands / hundred-
//! millions" entries in `digit-opts`.

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
