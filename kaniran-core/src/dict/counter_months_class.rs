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
//! Both defaults must be applied by the constructor at instantiation
//! time — they are not visible from the struct definition. The
//! `value-string` override ports alongside `find-counter` in a later
//! wave.
//!
//! Sole `def-special-counter` callsite: seq 1255430 — `:text` =
//! `"月"`, `:kana` = `"がつ"`.

use crate::dict::counter_text_class::CounterText;

#[derive(Debug, Clone)]
pub struct CounterMonths(pub CounterText);
