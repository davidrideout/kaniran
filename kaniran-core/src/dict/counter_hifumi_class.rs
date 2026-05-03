//! Port of `ichiran/dict:counter-hifumi` (`dict-counters.lisp:518`).
//!
//! Counter cache entry for the ~30 counters that take native
//! kun-yomi numeric prefixes (ひと, ふた, み, よ, いつ, む, なな,
//! や, ここの, と) for small counts instead of the Sino-Japanese
//! number readings. Adds one slot over [`CounterText`]:
//!
//! - `digit_set` — set of digit values for which the kun-yomi prefix
//!   applies. Values outside the set fall through to the parent's
//!   default reading.
//!
//! The Lisp class declares the slot with no default; every observed
//! `def-special-counter` callsite supplies `:digit-set` explicitly,
//! so the field is required at construction time. Most call sites
//! pass `[1, 2]` or `[1, 2, 3]`; the largest seen is `[1, 2, 3, 4, 5]`
//! (棹/竿).
//!
//! The `get-kana` override (table-driven prefix substitution) ports
//! alongside `find-counter` in a later wave.

use crate::dict::counter_text_class::CounterText;

#[derive(Debug, Clone)]
pub struct CounterHifumi {
    pub base: CounterText,
    pub digit_set: Vec<i32>,
}
