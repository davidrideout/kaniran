//! Port of `ichiran/dict:counter-halfhour` (`dict-counters.lisp:391`).
//!
//! Counter cache entry for 時半 (half past N o'clock), whose
//! `value-string` override formats the display string as `"N:30"`.

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
