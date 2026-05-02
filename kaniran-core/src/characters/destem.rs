//! Port of `ichiran/characters:destem` (`characters.lisp:316-324`).
//!
//! Trim from the end of `word` the suffix that begins at the
//! `stem`-th match of `char_class`'s pattern (counted from the end).
//! `stem == 0` returns `word` unchanged. When `word` has fewer than
//! `stem` matches of the class, the result is empty.
//!
//! Char-position semantics (CONVENTIONS §4.5): the cut is on a
//! character boundary, so multi-byte code points pass through
//! correctly. The compiled regex comes from
//! [`super::kani_char_class_bare_scanners::char_class_bare_scanners`].
//!
//! Diverges from the Lisp by exposing `char_class` as a required
//! argument (the Lisp `&optional` defaulted to `:kana`); both upstream
//! call-sites pass the default explicitly under this convention.

use super::char_class_type::CharClass;
use super::kani_char_class_bare_scanners::char_class_bare_scanners;

pub fn destem(word: &str, stem: usize, char_class: CharClass) -> String {
    if stem == 0 {
        return word.to_string();
    }
    let re = char_class_bare_scanners()
        .get(&char_class)
        .expect("char_class is in *char-class-regex-mapping*");
    let positions: Vec<usize> = re
        .find_iter(word)
        .map(|m| m.expect("regex iteration"))
        .map(|m| word[..m.start()].chars().count())
        .collect();
    if stem > positions.len() {
        return String::new();
    }
    let cut = positions[positions.len() - stem];
    word.chars().take(cut).collect()
}
