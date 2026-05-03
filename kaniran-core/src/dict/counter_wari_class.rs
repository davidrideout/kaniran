//! Port of `ichiran/dict:counter-wari` (`dict-counters.lisp:746`).
//!
//! Counter cache entry for 割 / 割引 (tenths / percentage). Adds no
//! slots over [`CounterText`]; the `value-string` override emits
//! `"N0%"` (since 1 割 == 10%). The kana surface comes through the
//! parent's `counter-join` unchanged.
//!
//! `def-special-counter` callsites:
//! - seq 1606800 — `:text` = `"割"`, `:kana` = `"わり"`.
//! - seq 1606950 — `:text` = `"割引"`, `:kana` = `"わりびき"`.
//!
//! The `value-string` override ports alongside `find-counter` in a
//! later wave.

use crate::dict::counter_text_class::CounterText;

#[derive(Debug, Clone)]
pub struct CounterWari(pub CounterText);
