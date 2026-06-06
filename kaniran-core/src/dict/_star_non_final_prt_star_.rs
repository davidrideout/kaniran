//! Port of `ichiran/dict:*non-final-prt*` (`dict-errata.lisp:1209`).
//!
//! Particles that don't get the final-position score bonus; the only
//! entry is `ん` (2139720).

pub static NON_FINAL_PRT: &[i32] = &[
    2139720, // ん
];
