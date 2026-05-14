//! Port of `ichiran/dict:gap-penalty` (`dict.lisp:1168`).
//!
//! ```lisp
//! (declaim (inline gap-penalty))
//! (defun gap-penalty (start end)
//!   (* (- end start) *gap-penalty*))
//! ```
//!
//! Score contribution for the unconsumed-character gap between two
//! segments (or between the input edge and the first/last segment).
//! Called from `find-best-path` at `dict.lisp:1193`, `:1202-1203`,
//! and `:1213-1214`. Negative result is the cost of a gap; a positive
//! result corresponds to overlapping endpoints (`end < start` — used
//! by the assembler to score overlapping splits).
//!
//! Subtraction is performed in `i64` so the negative-intermediate
//! case (`end < start`) doesn't underflow `usize`.

use crate::dict::_star_gap_penalty_star_::GAP_PENALTY;

pub fn gap_penalty(start: usize, end: usize) -> i64 {
    (end as i64 - start as i64) * GAP_PENALTY
}

#[cfg(test)]
mod tests {
    use super::*;

    // REPL-pinned (.103 SBCL 2.2.9, 2026-05-14):
    //   (ichiran/dict::gap-penalty 0 0)   => 0
    //   (ichiran/dict::gap-penalty 0 3)   => -1500
    //   (ichiran/dict::gap-penalty 7 9)   => -1000
    //   (ichiran/dict::gap-penalty 10 10) => 0
    //   (ichiran/dict::gap-penalty 5 2)   => 1500
    #[test]
    fn matches_repl() {
        assert_eq!(gap_penalty(0, 0), 0);
        assert_eq!(gap_penalty(0, 3), -1500);
        assert_eq!(gap_penalty(7, 9), -1000);
        assert_eq!(gap_penalty(10, 10), 0);
        assert_eq!(gap_penalty(5, 2), 1500);
    }
}
