//! Port of `ichiran/characters:*digit-regex*`
//! (`characters.lisp:128`).
//!
//! Matches one digit in either ASCII, full-width Latin, or the
//! single ideographic-zero glyph 〇.

pub static DIGIT_REGEX: &str = "[0-9０-９〇]";
