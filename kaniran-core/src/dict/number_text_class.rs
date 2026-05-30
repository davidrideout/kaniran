//! Port of `ichiran/dict:number-text` (`dict-counters.lisp:203`).
//!
//! Counter cache entry for the bare-number reading. Adds no slots over
//! [`crate::dict::counter_text_class::CounterText`]; only overrides
//! three slot defaults (`text` and `kana` to the empty string,
//! `ordinalp` to false). The populator seeds the cache with one of
//! these via `(add-args "" 'number-text)` before walking the readings
//! hash; `find-counter` instantiates per-query with `:number-text N`.
//!
//! Methods unique to this class (`get-kana` returning the kana for the
//! bare number, the inherited `value-string`, etc.) land alongside
//! `find-counter` in a later wave. This file only mirrors the slot
//! shape; the shared fields live on the wrapped `CounterText`.

use crate::dict::_star_kana_hint_space_star_::KANA_HINT_SPACE;
use crate::dict::counter_text_class::CounterText;
use crate::numbers::num_class::{DIGIT_KANJI_DEFAULT, POWER_KANJI};
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
