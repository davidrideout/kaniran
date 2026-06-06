//! Port of `ichiran/numbers:*digit-kanji-legal*` (`numbers.lisp:5`).
//!
//! Legal / financial 0–10 kanji glyph table, indexed by digit value.
//! These forms (`壱`, `弐`, `参`, `拾`, ...) are used in contracts and
//! formal contexts where the simpler `一`, `二`, `三`, `十` could be
//! altered.

pub const DIGIT_KANJI_LEGAL: &str = "〇壱弐参四五六七八九拾";
