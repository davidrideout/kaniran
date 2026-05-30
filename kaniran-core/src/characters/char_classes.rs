//! [`CharClass`] enum, the `*char-class-regex-mapping*` it keys, the
//! two compiled-scanner forms derived from it, and the composite
//! `*basic-split-regex*`. From `characters.lisp:131-155` (mapping +
//! scanners) and `:147` (`CharClass` itself, the `deftype`).

use std::collections::HashMap;
use std::sync::OnceLock;

use super::constants::{DECIMAL_POINT_REGEX, DIGIT_REGEX, NUM_WORD_REGEX, WORD_REGEX};

/// `(deftype char-class ...)` — variant order matches the upstream
/// `member` list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CharClass {
    Katakana,
    KatakanaUniq,
    Hiragana,
    Kanji,
    KanjiChar,
    Kana,
    Traditional,
    Nonword,
    Number,
}

/// `*char-class-regex-mapping*` — one-character pattern per class.
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

/// `*char-scanners*` — `^pat+$`.
pub fn char_scanners() -> &'static HashMap<CharClass, fancy_regex::Regex> {
    static CACHE: OnceLock<HashMap<CharClass, fancy_regex::Regex>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut h = HashMap::new();
        for (class, pat) in CHAR_CLASS_REGEX_MAPPING {
            let wrapped = format!("^{pat}+$");
            let re = fancy_regex::Regex::new(&wrapped)
                .unwrap_or_else(|e| panic!("class {class:?} pattern {wrapped:?} failed: {e}"));
            h.insert(*class, re);
        }
        h
    })
}

/// `*char-scanners-inner*` — `(?:pat)+`.
pub fn char_scanners_inner() -> &'static HashMap<CharClass, fancy_regex::Regex> {
    static CACHE: OnceLock<HashMap<CharClass, fancy_regex::Regex>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut h = HashMap::new();
        for (class, pat) in CHAR_CLASS_REGEX_MAPPING {
            let wrapped = format!("(?:{pat})+");
            let re = fancy_regex::Regex::new(&wrapped)
                .unwrap_or_else(|e| panic!("class {class:?} pattern {wrapped:?} failed: {e}"));
            h.insert(*class, re);
        }
        h
    })
}

/// `*basic-split-regex*` — composite tokenizer for
/// Japanese-mixed-with-digits text, built from the regex constants in
/// [`super::constants`] via the upstream `format` template.
pub fn basic_split_regex() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
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

/// `count-char-class` (`characters.lisp:165-170`) — non-overlapping
/// matches of `char_class`'s pattern in `word`.
pub fn count_char_class(word: &str, char_class: CharClass) -> usize {
    super::kani_char_class_bare_scanners::char_class_bare_scanners()
        .get(&char_class)
        .expect("char_class is in *char-class-regex-mapping*")
        .find_iter(word)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mapping_pattern_compiles_under_fancy_regex() {
        for (class, pat) in CHAR_CLASS_REGEX_MAPPING {
            fancy_regex::Regex::new(pat)
                .unwrap_or_else(|e| panic!("class {class:?} regex {pat:?} failed: {e}"));
        }
    }

    #[test]
    fn char_scanners_cover_every_class_in_the_mapping() {
        let h = char_scanners();
        for (class, _) in CHAR_CLASS_REGEX_MAPPING {
            assert!(h.contains_key(class), "missing scanner for {class:?}");
        }
    }

    #[test]
    fn char_scanners_inner_cover_every_class_in_the_mapping() {
        let h = char_scanners_inner();
        for (class, _) in CHAR_CLASS_REGEX_MAPPING {
            assert!(h.contains_key(class), "missing inner scanner for {class:?}");
        }
    }

    #[test]
    fn basic_split_regex_compiles_under_fancy_regex() {
        fancy_regex::Regex::new(basic_split_regex()).expect("regex must compile");
    }

    /// Pinned against the Lisp introspector's captured value.
    #[test]
    fn basic_split_regex_matches_introspected_value() {
        assert_eq!(
            basic_split_regex(),
            "((?:(?<![.,]|[0-9０-９〇])[0-9０-９〇]+|[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇])[0-9０-９〇々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー]*[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]|[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇])"
        );
    }
}
