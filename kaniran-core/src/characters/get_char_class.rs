//! Port of `ichiran/characters:get-char-class` (`characters.lisp:44-45`).
//!
//! Look up the [`KanaClass`] of a single character via
//! [`super::_star_char_class_hash_star_::char_class_hash`].
//!
//! The Lisp uses `(gethash char *char-class-hash* char)` — returning
//! the input character itself when the table doesn't know it. Per
//! CONVENTIONS §4.2 the Rust port returns `Option<KanaClass>` and
//! lets the caller fall back to the input they already have.

use super::_star_char_class_hash_star_::char_class_hash;
use super::kani_kana_class::KanaClass;

pub fn get_char_class(c: char) -> Option<KanaClass> {
    char_class_hash().get(&c).copied()
}
