//! Port of `ichiran/dict:gap-penalty` (`dict.lisp:1168`).
//!
//! Score contribution for the unconsumed-character gap between two
//! segments: `(end - start) * *gap-penalty*`. Negative is a gap cost;
//! positive (`end < start`) scores overlapping endpoints.

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
