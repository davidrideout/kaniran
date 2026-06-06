//! Port of `ichiran/dict:*identical-word-score-cutoff*` (`dict.lisp:1020`).
//!
//! Cutoff ratio `1/2` that `cull-segments` multiplies against the
//! max score to drop low-scoring identical-word segments.

pub const IDENTICAL_WORD_SCORE_CUTOFF: (i64, i64) = (1, 2);
