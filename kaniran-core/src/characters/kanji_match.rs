//! Port of `ichiran/characters:kanji-match` (`characters.lisp:200-201`).
//!
//! True iff `reading` matches the per-word regex built by
//! [`super::kanji_regex::kanji_regex`]. The Lisp returns the match
//! position (truthy) or `nil`; every caller uses it as a predicate, so
//! the Rust signature is `bool` per CONVENTIONS §4.1.

use super::kanji_regex::kanji_regex;

pub fn kanji_match(word: &str, reading: &str) -> bool {
    kanji_regex(word).is_match(reading).unwrap_or(false)
}
