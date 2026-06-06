//! Port of `ichiran/characters:get-char-class` (`characters.lisp:44-45`).
//!
//! Look up the [`KanaClass`] of a single character.

use super::_star_char_class_hash_star_::char_class_hash;
use super::kani_kana_class::KanaClass;

pub fn get_char_class(c: char) -> Option<KanaClass> {
    char_class_hash().get(&c).copied()
}
