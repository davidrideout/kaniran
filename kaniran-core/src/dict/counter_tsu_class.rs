//! Port of `ichiran/dict:counter-tsu` (`dict-counters.lisp:497`).
//!
//! Counter cache entry for the bare つ counter. Adds no slots over
//! [`CounterText`]. The `verify` override restricts validity to
//! `1 <= n <= 9`; the `get-kana` override is a closed table over
//! exactly those values (ひとつ, ふたつ, みっつ, よっつ, いつつ,
//! むっつ, ななつ, やっつ, ここのつ). Both methods port alongside
//! `find-counter` in a later wave.
//!
//! Sole `def-special-counter` callsite: seq 2220330 — `:text` =
//! `"つ"`, `:kana` = `"つ"`.

use crate::dict::counter_text_class::CounterText;

#[derive(Debug, Clone)]
pub struct CounterTsu(pub CounterText);

impl CounterTsu {
    /// `dict-counters.lisp:499` — `(<= 1 (number-value counter) 9)`
    /// AND `unique`. Ignores the `allowed` slot entirely; the bare つ
    /// counter is valid only for the kun-yomi 1..9 range and the
    /// `get-kana` table covers exactly those values.
    pub fn verify(&self, unique: bool) -> bool {
        let n = self.0.number;
        (1..=9).contains(&n) && unique
    }
}
