//! Port of `ichiran/dict:number-text` (`dict-counters.lisp:203`).
//!
//! Counter cache entry for the bare-number reading. Adds no slots over
//! [`crate::dict::counter_text_class::CounterText`]; only overrides
//! three slot defaults (`text` and `kana` to the empty string,
//! `ordinalp` to false). The populator seeds the cache with one of
//! these via `(add-args "" 'number-text)` before walking the readings
//! hash; `find-counter` instantiates per-query with `:number-text N`.
//!
//! Methods unique to this class (`get-kana` returning the kana for the
//! bare number, the inherited `value-string`, etc.) land alongside
//! `find-counter` in a later wave. This file only mirrors the slot
//! shape; the shared fields live on the wrapped `CounterText`.

use crate::dict::counter_text_class::CounterText;

#[derive(Debug, Clone)]
pub struct NumberText(pub CounterText);
