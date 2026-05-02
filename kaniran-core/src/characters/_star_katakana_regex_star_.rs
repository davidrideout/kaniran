//! Port of `ichiran/characters:*katakana-regex*`
//! (`characters.lisp:118`).
//!
//! Matches one katakana code point, including the iteration marks
//! ヽ ヾ and the long-vowel marker ー.

pub static KATAKANA_REGEX: &str = "[ァ-ヺヽヾー]";
