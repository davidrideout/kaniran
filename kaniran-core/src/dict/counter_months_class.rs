//! Port of `ichiran/dict:counter-months` (`dict-counters.lisp:721`).
//!
//! Counter cache entry for 月 read as がつ (month-of-year, January
//! through December). Adds no slots over [`CounterText`]; the
//! `value-string` override emits the English month name
//! (`"January"`..`"December"`) instead of the numeric default.
//!
//! Per-class slot defaults overridden by this Lisp class:
//! - `allowed` defaults to `[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]`
//!   (parent default is empty / no restriction).
//! - `digit_opts` defaults to
//!   `[(4, ["し"]), (7, ["しち"]), (9, ["く"])]` (parent default is
//!   empty).
//!
//! Both defaults are applied by the constructor at instantiation time
//! — they are not visible from the struct definition.
//!
//! Sole `def-special-counter` callsite: seq 1255430 — `:text` =
//! `"月"`, `:kana` = `"がつ"`.

use crate::dict::counter_text_class::CounterText;

#[derive(Debug, Clone)]
pub struct CounterMonths(pub CounterText);

const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March",
    "April", "May", "June",
    "July", "August", "September",
    "October", "November", "December",
];

impl CounterMonths {
    /// `value-string` override — `dict-counters.lisp:725-730`:
    ///
    /// ```lisp
    /// (defmethod value-string ((counter counter-months))
    ///   (aref #("January" ... "December") (1- (number-value counter))))
    /// ```
    ///
    /// Upstream `aref` raises a bounds error when `number-value` is
    /// outside `1..=12`; here that surfaces as a panic from the array
    /// index. Counter `verify` keeps `number` inside `allowed`
    /// (`[1..=12]` for this class) so the call site never reaches this
    /// method with an out-of-range value.
    pub fn value_string(&self) -> String {
        MONTH_NAMES[(self.0.number - 1) as usize].to_string()
    }
}
