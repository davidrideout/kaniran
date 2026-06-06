//! Port of `ichiran/characters:*katakana-uniq-regex*`
//! (`characters.lisp:119`).
//!
//! Matches one katakana code point, excluding the long-vowel marker ー
//! (which is shared with hiragana and so isn't unique to katakana).

pub static KATAKANA_UNIQ_REGEX: &str = "[ァ-ヺヽヾ]";
