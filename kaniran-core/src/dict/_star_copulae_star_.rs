//! Port of `ichiran/dict:*copulae*` (`dict-errata.lisp:1205`).
//!
//! JMdict seqs treated as copulae by `calc-score` (`dict.lisp:835`):
//! the segmenter intersects this list against the candidate's
//! `seq-set` to detect a copula `だ` and apply the cop-da
//! scoring branch.

pub static COPULAE: &[i32] = &[
    2089020, // だ
    // 2755350 // じゃない (commented out upstream)
];
