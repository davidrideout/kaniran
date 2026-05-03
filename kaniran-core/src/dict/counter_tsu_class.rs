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
