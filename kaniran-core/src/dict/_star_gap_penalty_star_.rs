//! Port of `ichiran/dict:*gap-penalty*` (`dict.lisp:1165`).
//!
//! ```lisp
//! (defparameter *gap-penalty* -500)
//! ```
//!
//! Per-character penalty multiplied into [`super::gap_penalty::gap_penalty`]
//! at `dict.lisp:1168` as `(* (- end start) *gap-penalty*)`. Applied
//! by the top-array assembler at `dict.lisp:1193`, `:1202-1203`, and
//! `:1213-1214` to score leading / trailing / inter-segment gaps in
//! `find-best-path`.
//!
//! Typed `i64` because the helper's product
//! `(end - start) * GAP_PENALTY` is the score-contribution scalar and
//! score arithmetic accumulates by `i64` for headroom (CL fixnums are
//! 62-bit signed).

pub const GAP_PENALTY: i64 = -500;
