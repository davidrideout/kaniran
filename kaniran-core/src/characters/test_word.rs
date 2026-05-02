//! Port of `ichiran/characters:test-word` (`characters.lisp:160-163`).
//!
//! True iff every character of `word` belongs to `char_class` — the
//! scanner from `*char-scanners*` is anchored as `^pat+$`, so any
//! non-class character makes the match fail. The Lisp returns the
//! match start position (truthy) or nil (falsy); every caller treats
//! it as a predicate, so the Rust signature is `bool`.

use super::_star_char_scanners_star_::char_scanners;
use super::char_class_type::CharClass;

pub fn test_word(word: &str, char_class: CharClass) -> bool {
    char_scanners()
        .get(&char_class)
        .expect("char_class is in *char-scanners*")
        .is_match(word)
        .unwrap_or(false)
}
