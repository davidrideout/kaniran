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

impl CounterDaysKun {
    /// `get-kana` override — `dict-counters.lisp:689-704`.
    /// Closed table over the allowed kun-yomi day-count values.
    /// Returns `Some` for every table entry; `Some(String::new())`
    /// for off-table values, mirroring upstream `case` without a
    /// `t` clause returning nil — which the `:around` then
    /// concatenates with the suffix as empty. Never falls through
    /// to `call-next-method` because the `verify` restriction
    /// limits inputs to the table entries.
    pub fn get_kana(&self) -> Option<String> {
        Some(match self.0.number as i64 {
            1 => "ついたち".to_string(),
            2 => "ふつか".to_string(),
            3 => "みっか".to_string(),
            4 => "よっか".to_string(),
            5 => "いつか".to_string(),
            6 => "むいか".to_string(),
            7 => "なのか".to_string(),
            8 => "ようか".to_string(),
            9 => "ここのか".to_string(),
            10 => "とうか".to_string(),
            14 => "じゅうよっか".to_string(),
            20 => "はつか".to_string(),
            24 => "にじゅうよっか".to_string(),
            30 => "みそか".to_string(),
            // `case` without `t` returns nil upstream;
            // `(concatenate 'string nil suffix)` treats nil as
            // empty. Mirror by emitting the empty string here.
            _ => String::new(),
        })
    }
}
