//! Port of `ichiran/numbers:*digit-kanji-default*` (`numbers.lisp:3`).
//!
//! Default 0–9 kanji glyph table, indexed by digit value. Used by
//! [`super::number_to_kanji::number_to_kanji`] to render an integer
//! into its everyday kanji form.

pub const DIGIT_KANJI_DEFAULT: &str = "〇一二三四五六七八九";
