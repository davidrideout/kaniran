//! Port of `ichiran/characters:*kanji-regex*`
//! (`characters.lisp:121`).
//!
//! Matches one kanji-ish code point: the CJK ideograph block
//! plus the iteration mark 々 and the abbreviation marks ヶ 〆.

pub static KANJI_REGEX: &str = "[々ヶ〆一-龯]";
