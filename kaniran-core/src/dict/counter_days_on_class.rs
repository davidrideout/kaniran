//! Port of `ichiran/dict:counter-days-on` (`dict-counters.lisp:709`).
//!
//! Counter cache entry for 日 read with the on-yomi day count にち.
//! Adds no slots over [`CounterText`]; the subclass exists for the
//! `verify` override that restricts validity to `n == 1` or
//! `n > 10` (and never 20 — those values belong to
//! [`crate::dict::counter_days_kun_class::CounterDaysKun`]). The
//! override ports alongside `find-counter` in a later wave.
//!
//! Sole `def-special-counter` callsite: seq 2083100 — `:text` =
//! `"日"`, `:kana` = `"にち"`.

use crate::dict::counter_text_class::CounterText;

#[derive(Debug, Clone)]
pub struct CounterDaysOn(pub CounterText);

impl CounterDaysOn {
    /// `dict-counters.lisp:711-716`:
    ///
    /// ```lisp
    /// (and (or (> n 10) (= n 1))
    ///      (/= n 20)
    ///      (call-next-method))
    /// ```
    ///
    /// `n` is the counter's number-value. The on-yomi day-count にち
    /// is valid for the literal day "1日" or any value above ten,
    /// excluding 20 (which is owned by [`CounterDaysKun`]).
    /// `(call-next-method)` falls through to [`CounterText::verify`].
    ///
    /// [`CounterDaysKun`]: crate::dict::counter_days_kun_class::CounterDaysKun
    pub fn verify(&self, unique: bool) -> bool {
        let n = self.0.number;
        (n > 10 || n == 1) && n != 20 && self.0.verify(unique)
    }
}
