//! Port of `ichiran/characters:*num-word-regex*`
//! (`characters.lisp:126`).
//!
//! Matches one character that can appear inside a mixed
//! number-and-word token: digits (ASCII or full-width), 〇, kanji,
//! katakana, or hiragana. Differs from
//! [`super::_star_word_regex_star_::WORD_REGEX`] only by also
//! admitting digits.

pub static NUM_WORD_REGEX: &str = "[0-9０-９〇々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー]";
