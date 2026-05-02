//! Port of `ichiran/characters:*nonword-regex*`
//! (`characters.lisp:124`).
//!
//! Matches one character that is NOT part of a word as ichiran
//! defines it: complement of kanji + iteration/abbreviation marks
//! + katakana + hiragana + ideographic zero.

pub static NONWORD_REGEX: &str = "[^々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]";
