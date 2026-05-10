//! Port of `ichiran/dict:counter-halfhour` (`dict-counters.lisp:391`).
//!
//! Counter cache entry for 時半 (half past N o'clock). Adds no slots
//! over [`CounterText`]; only override is the `value-string` method
//! which formats the display string as `"N:30"` instead of the
//! default `"Value: N"`. The kana surface comes through the parent's
//! `counter-join` unchanged.
//!
//! Sole `def-special-counter` callsite: seq 1658480 — `:text` =
//! `"時半"`, `:kana` = `"じはん"`,
//! `:digit-opts` = `[(4 ["よ"]), (9 ["く"])]`.

use crate::dict::counter_text_class::CounterText;

#[derive(Debug, Clone)]
pub struct CounterHalfhour(pub CounterText);

impl CounterHalfhour {
    /// `value-string` override — `dict-counters.lisp:393-394`:
    ///
    /// ```lisp
    /// (defmethod value-string ((counter counter-halfhour))
    ///   (format nil "~a:30" (number-value counter)))
    /// ```
    pub fn value_string(&self) -> String {
        format!("{}:30", self.0.number)
    }
}
