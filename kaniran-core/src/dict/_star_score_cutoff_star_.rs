//! Port of `ichiran/dict:*score-cutoff*` (`dict.lisp:1069`).
//!
//! ```lisp
//! (defparameter *score-cutoff* 5) ;; this must filter out ONLY bad kana spellings, and NOT filter out any kanji spellings
//! ```
//!
//! Consulted at two `dict.lisp` sites:
//! - `kanji-break-penalty` at `dict.lisp:730-731` —
//!   `(if (>= score *score-cutoff*) (max *score-cutoff* (+ (ceiling score ratio) bonus)) score)`.
//! - `join-substring-words` at `dict.lisp:1125` —
//!   `if (>= (segment-score segment) *score-cutoff*) collect segment`.
//!
//! Typed `i32` to match the `score: Option<i32>` field on
//! [`super::segment_struct::Segment`].
//!
//! ## Known overflow-domain divergence
//!
//! Upstream scores are CL fixnums (62-bit signed). The Rust port
//! tracks `Segment.score` as `i32`, accepted because observed values
//! during fixture replay stay well inside `i32::MAX`. `*score-cutoff*`
//! is matched to that type — i.e. inherits the same caveat. If a
//! future score arithmetic path (`gen-score` × `length-multiplier-coeff`
//! × `kanji-break-penalty` chains) saturates `i32`, the fix is to
//! widen `Segment.score` to `i64` and propagate; this constant moves
//! with it.

pub const SCORE_CUTOFF: i32 = 5;
