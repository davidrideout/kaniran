//! Port of `ichiran/dict:*score-cutoff*` (`dict.lisp:1069`).
//!
//! Minimum segment score (5) used to filter out bad kana spellings
//! without dropping any kanji spellings.

pub const SCORE_CUTOFF: i32 = 5;
