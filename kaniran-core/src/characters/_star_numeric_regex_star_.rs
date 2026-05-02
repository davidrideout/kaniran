//! Port of `ichiran/characters:*numeric-regex*`
//! (`characters.lisp:125`).
//!
//! Matches one numeric character: ASCII digit, full-width digit,
//! ideographic zero 〇, kanji-numeral (一-九), or one of the
//! traditional / large-unit numeral kanji used in Japanese
//! number-writing (零壱弐参拾十百千万億兆京).

pub static NUMERIC_REGEX: &str = "[0-9０-９〇一二三四五六七八九零壱弐参拾十百千万億兆京]";
