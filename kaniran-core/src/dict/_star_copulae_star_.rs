//! Port of `ichiran/dict:*copulae*` (`dict-errata.lisp:1205`).
//!
//! JMdict seqs treated as copulae (e.g. だ) during scoring.

pub static COPULAE: &[i32] = &[
    2089020, // だ
    // 2755350 // じゃない (commented out upstream)
];
