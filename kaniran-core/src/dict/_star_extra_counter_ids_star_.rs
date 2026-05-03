//! Port of `ichiran/dict:*extra-counter-ids*` (`dict-counters.lisp:310`).
//!
//! JMdict seqs that the project treats as counters even though
//! JMdict itself does not tag them with `pos=ctr`. The `get-counter-readings`
//! pipeline appends these to the SQL "pos=ctr" sweep before
//! [`crate::dict::_star_skip_counter_ids_star_::SKIP_COUNTER_IDS`]
//! is subtracted.

pub static EXTRA_COUNTER_IDS: &[i32] = &[
    1255430, // 月
    1606800, // 割
];
