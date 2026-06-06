//! Port of `ichiran/dict:*extra-counter-ids*` (`dict-counters.lisp:310`).
//!
//! JMdict seqs treated as counters even though JMdict itself does not
//! tag them with `pos=ctr`.

pub static EXTRA_COUNTER_IDS: &[i32] = &[
    1255430, // 月
    1606800, // 割
];
