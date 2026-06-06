//! Port of `ichiran/dict:length-multiplier` (`dict.lisp:681`).
//!
//! Returns `length^power` while `length <= len-lim`, otherwise goes
//! linear with `length * len-lim^(power-1)`.

pub fn length_multiplier(length: i64, power: i64, len_lim: i64) -> i64 {
    if length <= len_lim {
        length.pow(power as u32)
    } else {
        length * len_lim.pow((power - 1) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // REPL fixtures (.103, ichiran/dict::length-multiplier), 2026-05-25.
    // `(length, power, len-lim) -> result`; both cond branches and the
    // `length == len-lim` boundary (first branch) are covered.
    #[test]
    fn length_multiplier_fixtures() {
        let cases: &[(i64, i64, i64, i64)] = &[
            // length <= len-lim  → length^power
            (3, 2, 5, 9),
            (5, 2, 5, 25), // boundary: length == len-lim
            (4, 3, 6, 64),
            (3, 1, 5, 3),
            (1, 4, 2, 1),
            // length > len-lim   → length * len-lim^(power-1)
            (7, 2, 5, 35),
            (8, 3, 6, 288),
            (7, 1, 5, 7), // power 1 → len-lim^0 = 1
            (10, 2, 3, 30),
            (6, 4, 4, 384),
        ];
        for &(length, power, len_lim, expected) in cases {
            assert_eq!(
                length_multiplier(length, power, len_lim),
                expected,
                "length={length} power={power} len_lim={len_lim}"
            );
        }
    }
}
