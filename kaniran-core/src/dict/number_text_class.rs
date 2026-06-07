//! Port of `ichiran/dict:number-text` (`dict-counters.lisp:203`).
//!
//! Counter cache entry for the bare-number reading; adds no slots over
//! [`crate::dict::counters::classes::CounterText`].

use crate::dict::split::segsplit::KANA_HINT_SPACE;
use crate::dict::counters::classes::CounterText;
use crate::numbers::constants::DIGIT_KANJI_DEFAULT;
use crate::numbers::constants::POWER_KANJI;
use crate::numbers::kana_form::{number_to_kana, NumberToKanaOutput};
use crate::numbers::kanji_form::number_to_kanji;

#[derive(Debug, Clone)]
pub struct NumberText(pub CounterText);

impl NumberText {
    /// `get-kana` override — `dict-counters.lisp:208-209`:
    /// `(number-to-kana (number-value obj) :separator *kana-hint-space*)`.
    /// Always specializes; no call-next-method path.
    pub fn get_kana(&self) -> String {
        match number_to_kana(self.0.number, Some(KANA_HINT_SPACE), |x| {
            number_to_kanji(x, DIGIT_KANJI_DEFAULT, POWER_KANJI, false)
        }) {
            NumberToKanaOutput::Joined(s) => s,
            NumberToKanaOutput::Groups(_) => unreachable!(
                "number-to-kana with Some(separator) always returns Joined"
            ),
        }
    }
}
