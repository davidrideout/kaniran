//! Port of `ichiran/characters:*kanji-char-regex*`
//! (`characters.lisp:122`).
//!
//! Matches one CJK Unified Ideograph in the U+4E00…U+9FAF range.
//! Strict version of [`super::_star_kanji_regex_star_::KANJI_REGEX`]
//! that excludes the iteration mark 々 and the abbreviation marks
//! ヶ 〆.

pub static KANJI_CHAR_REGEX: &str = "[一-龯]";
