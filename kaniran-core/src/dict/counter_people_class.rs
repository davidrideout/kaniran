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

impl CounterPeople {
    /// `get-kana` override — `dict-counters.lisp:737-741`. Returns
    /// ひとり for 1, ふたり for 2, otherwise `None` to fall through
    /// to the counter-text base primary.
    pub fn get_kana(&self) -> Option<String> {
        match self.0.number as i64 {
            1 => Some("ひとり".to_string()),
            2 => Some("ふたり".to_string()),
            _ => None,
        }
    }
}
