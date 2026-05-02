//! Port of `ichiran/characters:*decimal-point-regex*`
//! (`characters.lisp:129`).
//!
//! One of the small pattern-string parts that compose
//! `*basic-split-regex*`; kept as a raw pattern here for symmetry.

pub static DECIMAL_POINT_REGEX: &str = "[.,]";
