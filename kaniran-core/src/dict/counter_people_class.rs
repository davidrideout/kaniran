//! Port of `ichiran/dict:counter-people` (`dict-counters.lisp:735`).
//!
//! Counter cache entry for 人 (person count). Adds no slots over
//! [`CounterText`]; the `get-kana` override returns ひとり for 1 and
//! ふたり for 2, falling through to the default for all other counts.
//!
//! Sole `def-special-counter` callsite: seq 2149890 — `:text` =
//! `"人"`, `:kana` = `"にん"`,
//! `:digit-opts` = `[(4 ["よ"]), (7 ["しち"])]`,
//! `:accepts` = `[:chuu]`. The `get-kana` override ports alongside
//! `find-counter` in a later wave.

use crate::dict::counter_text_class::CounterText;

#[derive(Debug, Clone)]
pub struct CounterPeople(pub CounterText);
