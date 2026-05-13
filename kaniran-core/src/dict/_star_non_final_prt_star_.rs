//! Port of `ichiran/dict:*non-final-prt*` (`dict-errata.lisp:1209`).
//!
//! "Particles that don't get final bonus" — read by `calc-score`
//! (`dict.lisp:833`) as `(member seq *non-final-prt*)` so the
//! particle does not receive the final-position score bump.
//! The only entry is the sentence-final-but-not-bonus particle
//! `ん` (2139720).

pub static NON_FINAL_PRT: &[i32] = &[
    2139720, // ん
];
