//! Port of `ichiran/characters:*katakana-uniq-regex*`
//! (`characters.lisp:119`).
//!
//! Like [`super::_star_katakana_regex_star_::KATAKANA_REGEX`] but
//! excludes the long-vowel marker ー (which is shared with hiragana
//! and so isn't unique to katakana).

pub static KATAKANA_UNIQ_REGEX: &str = "[ァ-ヺヽヾ]";
