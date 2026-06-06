//! Port of `ichiran/characters:kanji-match` (`characters.lisp:200-201`).
//!
//! True iff `reading` matches the per-word regex built by
//! [`super::kanji_regex::kanji_regex`].

use super::kanji_regex::kanji_regex;

pub fn kanji_match(word: &str, reading: &str) -> bool {
    kanji_regex(word).is_match(reading).unwrap_or(false)
}
