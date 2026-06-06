//! Port of `ichiran/dict:counter-wari` (`dict-counters.lisp:746`).
//!
//! Counter cache entry for 割 / 割引 (tenths / percentage), whose
//! `value-string` override emits `"N0%"` (since 1 割 == 10%).

use crate::dict::counter_text_class::CounterText;

#[derive(Debug, Clone)]
pub struct CounterWari(pub CounterText);

impl CounterWari {
    /// `value-string` override — `dict-counters.lisp:748-749`:
    ///
    /// ```lisp
    /// (defmethod value-string ((counter counter-wari))
    ///   (format nil "~a%" (* 10 (number-value counter))))
    /// ```
    pub fn value_string(&self) -> String {
        format!("{}%", self.0.number * 10)
    }
}
