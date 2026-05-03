//! Port of `ichiran/dict:counter-age` (`dict-counters.lisp:757`).
//!
//! Counter cache entry for the 歳 / 才 (age) counter. Adds no slots
//! over [`CounterText`]; the subclass exists only so the dispatcher
//! can route to the `get-kana` override that turns 20 into はたち
//! (every other value falls through to the default).
//!
//! Sole `def-special-counter` callsite: seq 1294940 — `:text` =
//! `"歳"` or `"才"`, `:kana` = `"さい"`. The `get-kana` override
//! ports alongside `find-counter` in a later wave.

use crate::dict::counter_text_class::CounterText;

#[derive(Debug, Clone)]
pub struct CounterAge(pub CounterText);
