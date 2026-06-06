//! Port of `ichiran/dict:*no-kanji-break*` (`dict-errata.lisp:1229`).
//!
//! Literal substrings that do not cause a kanji break in the segmenter.

pub static NO_KANJI_BREAK: &[&str] = &["日置"];
