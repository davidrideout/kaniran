//! Port of `ichiran/dict:*no-kanji-break*` (`dict-errata.lisp:1229`).
//!
//! ```lisp
//! (defparameter *no-kanji-break*
//!   '("日置")  ;; problematic with 一日置く
//!   "Words that do not cause kanji break")
//! ```
//!
//! Consulted by `dict.lisp:1105` as
//! `(find part *no-kanji-break* :test 'equal)` — when a candidate
//! substring matches one of these literals, the segmenter suppresses
//! the kanji break that would otherwise apply.

pub static NO_KANJI_BREAK: &[&str] = &["日置"];
