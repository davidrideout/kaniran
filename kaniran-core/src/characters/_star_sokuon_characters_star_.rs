//! Port of `ichiran/characters:*sokuon-characters*`
//! (`characters.lisp:3`).
//!
//! Single-entry table for the geminating mark (small tsu, hiragana
//! `っ` / katakana `ッ`). One of the four constituents that
//! `*all-characters*` is built from.

use super::kani_kana_class::KanaClass;

pub static SOKUON_CHARACTERS: &[(KanaClass, &str)] = &[(KanaClass::Sokuon, "っッ")];
