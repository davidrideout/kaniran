//! Port of `ichiran/characters:*word-regex*`
//! (`characters.lisp:127`).
//!
//! Matches one character that can appear inside a Japanese word:
//! kanji + iteration/abbreviation marks + katakana + hiragana +
//! ideographic zero.

pub static WORD_REGEX: &str = "[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]";
