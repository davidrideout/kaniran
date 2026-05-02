//! Port of `ichiran/characters:*word-regex*`
//! (`characters.lisp:127`).
//!
//! Matches one character that can appear inside a Japanese word:
//! kanji + iteration/abbreviation marks + katakana + hiragana +
//! ideographic zero. Same set as the kanji+kana characters
//! enumerated in [`super::_star_kanji_regex_star_::KANJI_REGEX`],
//! [`super::_star_katakana_regex_star_::KATAKANA_REGEX`], and
//! [`super::_star_hiragana_regex_star_::HIRAGANA_REGEX`], plus 〇.

pub static WORD_REGEX: &str = "[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]";
