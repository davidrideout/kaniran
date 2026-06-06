//! Port of `ichiran/dict:counter-age` (`dict-counters.lisp:757`).
//!
//! Counter cache entry for the 歳 / 才 (age) counter, whose `get-kana`
//! override turns 20 into はたち (every other value falls through to
//! the default).

use crate::dict::counter_text_class::CounterText;

#[derive(Debug, Clone)]
pub struct CounterAge(pub CounterText);

impl CounterAge {
    /// `get-kana` override — `dict-counters.lisp:759-762`. Returns
    /// はたち for 20, otherwise `None` to fall through to the
    /// counter-text base primary.
    pub fn get_kana(&self) -> Option<String> {
        match self.0.number as i64 {
            20 => Some("はたち".to_string()),
            _ => None,
        }
    }
}
