//! Port of `ichiran/characters:as-katakana` (`characters.lisp:262-271`).
//!
//! Mirror of [`super::as_hiragana`]: each kana char is replaced with
//! the *last* glyph of its class string in `*all-characters*` (each
//! pair is hiragana-then-katakana, so the last char is katakana).
//! Non-kana characters pass through unchanged.

use super::kana_class_tables::{all_characters, char_class_hash};
use super::kani_kana_class::KanaClass;
use super::to_normal_char::{to_normal_char, NormalizationContext};

pub fn as_katakana(s: &str) -> String {
    s.chars()
        .map(|c| {
            let c = to_normal_char(c, NormalizationContext::Default).unwrap_or(c);
            match char_class_hash().get(&c) {
                Some(&class) => last_char_for(class),
                None => c,
            }
        })
        .collect()
}

fn last_char_for(class: KanaClass) -> char {
    all_characters()
        .iter()
        .find(|(k, _)| *k == class)
        .expect("class from char-class-hash must be in all-characters")
        .1
        .chars()
        .last()
        .expect("class string is non-empty")
}
