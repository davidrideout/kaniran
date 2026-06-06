//! Port of `ichiran/dict:*segment-score-cutoff*` (`dict.lisp:1351`).
//!
//! Threshold ratio `2/3` that `word-info-from-segment-list` multiplies
//! against the max score to drop low-scoring segments.

pub const SEGMENT_SCORE_CUTOFF: (i64, i64) = (2, 3);
