//! Port of `ichiran/characters:*hiragana-regex*`
//! (`characters.lisp:120`).
//!
//! Matches one hiragana code point, including the iteration marks
//! ゝ ゞ and the long-vowel marker ー.

pub static HIRAGANA_REGEX: &str = "[ぁ-ゔゝゞー]";
