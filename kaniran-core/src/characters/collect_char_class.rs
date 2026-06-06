//! Port of `ichiran/characters:collect-char-class` (`characters.lisp:172-177`).
//!
//! Collect every non-overlapping match of `char_class`'s pattern from
//! `*char-class-regex-mapping*` in `word`, in left-to-right order.

use super::char_class_type::CharClass;
use super::kani_char_class_bare_scanners::char_class_bare_scanners;

pub fn collect_char_class(word: &str, char_class: CharClass) -> Vec<String> {
    char_class_bare_scanners()
        .get(&char_class)
        .expect("char_class is in *char-class-regex-mapping*")
        .find_iter(word)
        .map(|m| m.expect("regex iteration").as_str().to_string())
        .collect()
}
