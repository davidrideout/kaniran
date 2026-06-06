//! Port of `ichiran/dict:*gap-penalty*` (`dict.lisp:1165`).
//!
//! Per-character score penalty applied to gaps when scoring a path.

pub const GAP_PENALTY: i64 = -500;
