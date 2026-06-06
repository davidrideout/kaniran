//! Port of `ichiran/dict:*force-kanji-break*` (`dict-errata.lisp:1226`).
//!
//! Literal substrings that force the segmenter to break at a kanji
//! boundary.

pub static FORCE_KANJI_BREAK: &[&str] = &["です"];
