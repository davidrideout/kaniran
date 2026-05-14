//! Port of `ichiran/dict:*identical-word-score-cutoff*` (`dict.lisp:1020`).
//!
//! ```lisp
//! (defparameter *identical-word-score-cutoff* 1/2)
//! ```
//!
//! Cutoff ratio applied at `dict.lisp:1033` inside `cull-segments` as
//! `(cutoff (* max-score *identical-word-score-cutoff*))`, then the
//! loop keeps segments with `(>= (segment-score seg) cutoff)`. CL
//! evaluates the multiplication and comparison in **exact rational
//! arithmetic** — `(* 1/2 max-score)` stays as a ratio and the
//! comparison is `(>= seg-score cutoff)` over exact rationals.
//!
//! Stored here as the numerator / denominator pair `(1, 2)` so the
//! eventual `cull-segments` port can compare via integer
//! cross-multiplication (`DEN * seg_score >= NUM * max_score`),
//! mirroring the [`super::_star_segment_score_cutoff_star_`] (`2/3`)
//! precedent where exact `f64` representation isn't available. `1/2`
//! itself is exact under IEEE-754, but keeping the integer-pair shape
//! lets both cutoffs share the same comparator and audit harness.

pub const IDENTICAL_WORD_SCORE_CUTOFF: (i64, i64) = (1, 2);
