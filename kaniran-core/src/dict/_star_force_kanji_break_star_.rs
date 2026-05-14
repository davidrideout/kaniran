//! Port of `ichiran/dict:*force-kanji-break*` (`dict-errata.lisp:1226`).
//!
//! ```lisp
//! (defparameter *force-kanji-break*
//!   '("です"))
//! ```
//!
//! Consulted by `dict.lisp:1103` as
//! `(find part *force-kanji-break* :test 'equal)` — when a candidate
//! substring matches one of these literals, the segmenter forces a
//! kanji break at that position.

pub static FORCE_KANJI_BREAK: &[&str] = &["です"];
