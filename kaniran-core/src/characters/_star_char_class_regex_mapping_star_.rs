//! Port of `ichiran/characters:*char-class-regex-mapping*`
//! (`characters.lisp:136`).
//!
//! Mapping from a [`CharClass`] to a regex string that matches one
//! character of that class.

use super::char_class_type::CharClass;

pub static CHAR_CLASS_REGEX_MAPPING: &[(CharClass, &str)] = &[
    (CharClass::Katakana, "[ァ-ヺヽヾー]"),
    (CharClass::KatakanaUniq, "[ァ-ヺヽヾ]"),
    (CharClass::Hiragana, "[ぁ-ゔゝゞー]"),
    (CharClass::Kanji, "[々ヶ〆一-龯]"),
    (CharClass::KanjiChar, "[一-龯]"),
    (CharClass::Kana, "([ァ-ヺヽヾー]|[ぁ-ゔゝゞー])"),
    (CharClass::Traditional, "([ぁ-ゔゝゞー]|[々ヶ〆一-龯])"),
    (CharClass::Nonword, "[^々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]"),
    (
        CharClass::Number,
        "[0-9０-９〇一二三四五六七八九零壱弐参拾十百千万億兆京]",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pattern_compiles_under_fancy_regex() {
        for (class, pat) in CHAR_CLASS_REGEX_MAPPING {
            fancy_regex::Regex::new(pat)
                .unwrap_or_else(|e| panic!("class {class:?} regex {pat:?} failed: {e}"));
        }
    }
}
