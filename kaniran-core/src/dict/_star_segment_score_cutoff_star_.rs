//! Port of `ichiran/dict:*segment-score-cutoff*` (`dict.lisp:1351`).
//!
//! ```lisp
//! (defparameter *segment-score-cutoff* 2/3)
//! ```
//!
//! Threshold ratio applied at `dict.lisp:1360` as
//! `(< wi-score (* *segment-score-cutoff* max-score))`. CL evaluates
//! the multiplication and comparison in **exact rational arithmetic**
//! — `(* 2/3 1000)` is `2000/3`, and `(< wi-score 2000/3)` is true iff
//! `(< (* 3 wi-score) 2000)`. No coercion to float happens upstream.
//!
//! Stored here as the numerator / denominator pair `(2, 3)` so the
//! eventual `word-info-from-segment-list` port can compare via
//! integer cross-multiplication (`DEN * wi_score < NUM * max_score`)
//! and stay bit-identical to the Lisp at boundary scores. An `f64`
//! literal `2.0 / 3.0` would round at boundaries that the Lisp does
//! not.

pub const SEGMENT_SCORE_CUTOFF: (i64, i64) = (2, 3);
