//! Port of `ichiran/characters:*basic-split-regex*`
//! (`characters.lisp:131`).
//!
//! Composite pattern that recognizes one "word" token in
//! Japanese-mixed-with-digits text.

use std::sync::OnceLock;

use super::_star_decimal_point_regex_star_::DECIMAL_POINT_REGEX;
use super::_star_digit_regex_star_::DIGIT_REGEX;
use super::_star_num_word_regex_star_::NUM_WORD_REGEX;
use super::_star_word_regex_star_::WORD_REGEX;

static CACHE: OnceLock<String> = OnceLock::new();

pub fn basic_split_regex() -> &'static str {
    CACHE.get_or_init(|| {
        format!(
            "((?:(?<!{}|{}){}+|{}){}*{}|{})",
            DECIMAL_POINT_REGEX,
            DIGIT_REGEX,
            DIGIT_REGEX,
            WORD_REGEX,
            NUM_WORD_REGEX,
            WORD_REGEX,
            WORD_REGEX,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_under_fancy_regex() {
        fancy_regex::Regex::new(basic_split_regex()).expect("regex must compile");
    }

    #[test]
    fn matches_introspected_value() {
        assert_eq!(
            basic_split_regex(),
            "((?:(?<![.,]|[0-9０-９〇])[0-9０-９〇]+|[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇])[0-9０-９〇々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー]*[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]|[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇])"
        );
    }
}
