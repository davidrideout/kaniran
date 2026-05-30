//! Port of `ichiran/characters:as-hiragana` (`characters.lisp:251-260`).
//!
//! Convert any katakana in `s` to its hiragana counterpart, leaving
//! non-kana characters as-is. Each char is first run through
//! [`to_normal_char`] (default context) to normalize half/full-width
//! forms; the result is then looked up in `*char-class-hash*` and
//! replaced with the hiragana glyph for that class (the first char of
//! the class's `*all-characters*` string).

use super::kana_class_tables::{all_characters, char_class_hash};
use super::kani_kana_class::KanaClass;
use super::to_normal_char::{to_normal_char, NormalizationContext};

pub fn as_hiragana(s: &str) -> String {
    s.chars()
        .map(|c| {
            let c = to_normal_char(c, NormalizationContext::Default).unwrap_or(c);
            match char_class_hash().get(&c) {
                Some(&class) => first_char_for(class),
                None => c,
            }
        })
        .collect()
}

fn first_char_for(class: KanaClass) -> char {
    all_characters()
        .iter()
        .find(|(k, _)| *k == class)
        .expect("class from char-class-hash must be in all-characters")
        .1
        .chars()
        .next()
        .expect("class string is non-empty")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn katakana_becomes_hiragana_kanji_passes_through() {
        assert_eq!(as_hiragana("ア"), "あ");
        assert_eq!(as_hiragana("カタカナ"), "かたかな");
        assert_eq!(as_hiragana("日本ア"), "日本あ");
    }
}
