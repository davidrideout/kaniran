//! Port of `ichiran/characters:count-char-class` (`characters.lisp:165-170`).
//!
//! Count non-overlapping matches of `char_class`'s pattern from
//! `*char-class-regex-mapping*` in `word`. The compiled regex is
//! cached in [`super::kani_char_class_bare_scanners`].

use super::char_class_type::CharClass;
use super::kani_char_class_bare_scanners::char_class_bare_scanners;

pub fn count_char_class(word: &str, char_class: CharClass) -> usize {
    char_class_bare_scanners()
        .get(&char_class)
        .expect("char_class is in *char-class-regex-mapping*")
        .find_iter(word)
        .count()
}
