//! Port of `ichiran/dict:counter-days-kun` (`dict-counters.lisp:686`).
//!
//! Counter cache entry for 日 read with the kun-yomi day counts
//! (ついたち, ふつか, みっか, よっか, いつか, むいか, なのか,
//! ようか, ここのか, とうか, じゅうよっか, はつか, にじゅうよっか,
//! みそか). Adds no slots over [`CounterText`]; the `get-kana`
//! override is a closed table over the allowed values and ports
//! alongside `find-counter` in a later wave.
//!
//! Per-class slot default overridden by this Lisp class:
//! - `allowed` defaults to
//!   `[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 14, 20, 24, 30]` (parent
//!   default is empty / no restriction).
//!
//! That default must be applied by the constructor at instantiation
//! time — it is not visible from the struct definition.
//!
//! Sole `def-special-counter` callsite: seq 2083110 — `:text` =
//! `"日"`, `:kana` = `"か"`, `:common` = `0`, `:accepts` = `[:kan]`.

use crate::dict::counter_text_class::CounterText;

#[derive(Debug, Clone)]
pub struct CounterDaysKun(pub CounterText);
