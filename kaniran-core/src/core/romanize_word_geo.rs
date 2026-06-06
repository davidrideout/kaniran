//! Port of `ichiran:romanize-word-geo` (`romanize.lisp:232-233`).
//!
//! Romanizes `input` (normalized) and capitalizes the result, for place
//! names.

use unicode_properties::{GeneralCategory, GeneralCategoryGroup, UnicodeGeneralCategory};

use super::generic_romanization_class::RomanizationMethod;
use super::romanize_word::romanize_word;

pub fn romanize_word_geo(input: &str, method: RomanizationMethod<'_>) -> String {
    string_capitalize(&romanize_word(input, method, None, true))
}

/// `string-capitalize` (CL builtin): upcase the first cased character of
/// every alphanumeric-delimited word and downcase the rest; a
/// non-alphanumeric character passes through unchanged and starts a new
/// word.
fn string_capitalize(string: &str) -> String {
    let mut out = String::with_capacity(string.len());
    let mut newword = true;
    for char in string.chars() {
        if !alphanumericp(char) {
            out.push(char);
            newword = true;
        } else if newword {
            out.extend(char.to_uppercase());
            newword = false;
        } else {
            out.extend(char.to_lowercase());
        }
    }
    out
}

/// `(alphanumericp char)` — letter categories (Lu, Ll, Lt, Lm, Lo) or
/// decimal-number (Nd). Std `char::is_alphanumeric` diverges by also
/// counting Nl and No, which `alphanumericp` rejects.
fn alphanumericp(char: char) -> bool {
    char.general_category_group() == GeneralCategoryGroup::Letter
        || char.general_category() == GeneralCategory::DecimalNumber
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::_star_hepburn_simple_star_::hepburn_simple;
    use crate::core::_star_hepburn_traditional_star_::hepburn_traditional;

    #[test]
    fn romanize_word_geo_fixtures() {
        // REPL fixtures (.103, ichiran:romanize-word-geo), 2026-05-24.
        // Real Japanese place names; default method *hepburn-simple*.
        let simple = RomanizationMethod::SimplifiedHepburn(hepburn_simple());
        let cases: &[(&str, &str)] = &[
            ("とうきょう", "Tokyo"),
            ("おおさか", "Osaka"),
            ("ほっかいどう", "Hokkaido"),
            ("ぐんま", "Gunma"),
            // ん before お yields the apostrophe boundary; string-capitalize
            // upcases the vowel after the apostrophe ("shin'osaka" → "Shin'Osaka").
            ("しんおおさか", "Shin'Osaka"),
            ("きょうと", "Kyoto"),
            ("ふじさん", "Fujisan"),
            ("しんじゅく", "Shinjuku"),
            // r-special: lone small tsu / long-vowel bar pass through capitalize.
            ("っ", "!"),
            ("ー", "~"),
            // empty input
            ("", ""),
            // half-width katakana normalizes to full width before romanizing
            ("ﾄｳｷｮｳ", "Tokyo"),
            // kanji is not in the kana table, so it passes through unchanged
            ("東京", "東京"),
            ("ニューヨーク", "Nyuyoku"),
        ];
        for (input, expected) in cases {
            assert_eq!(&romanize_word_geo(input, simple), expected, "input={input}");
        }
    }

    #[test]
    fn romanize_word_geo_method_param() {
        // REPL fixtures (.103, (romanize-word-geo W :method *hepburn-traditional*)),
        // 2026-05-24 — the method keyword overrides the hepburn-simple default,
        // here producing macron long vowels.
        let traditional = RomanizationMethod::TraditionalHepburn(hepburn_traditional());
        assert_eq!(romanize_word_geo("とうきょう", traditional), "Tōkyō");
        assert_eq!(romanize_word_geo("おおさか", traditional), "Ōsaka");
    }

    #[test]
    fn string_capitalize_fixtures() {
        // REPL fixtures (.103, cl:string-capitalize), 2026-05-24 — exercises
        // both `newword` states, the alphanumeric branch, and passthrough.
        let cases: &[(&str, &str)] = &[
            // apostrophe word boundary
            ("shin'osaka", "Shin'Osaka"),
            ("n'pou", "N'Pou"),
            // space-delimited words; trailing letters downcased
            ("hello world", "Hello World"),
            ("ABC DEF", "Abc Def"),
            // interior digits do not break the word
            ("abc123def", "Abc123def"),
            ("a5b", "A5b"),
            // leading digit is alphanumeric but uncased; the run stays one word
            ("5abc", "5abc"),
            // hyphen is not alphanumeric, so it starts a new word
            ("foo-bar", "Foo-Bar"),
            // non-alphanumeric-only and empty inputs
            ("!", "!"),
            ("~", "~"),
            ("", ""),
            // ideographs are alphanumeric (Lo) but have no case change
            ("東京", "東京"),
        ];
        for (input, expected) in cases {
            assert_eq!(&string_capitalize(input), expected, "input={input}");
        }
    }
}
