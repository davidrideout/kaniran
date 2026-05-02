//! Port of the `ichiran/characters` Lisp package.
//!
//! One file per ported symbol; file paths follow the rule in
//! [`crate::kani::naming`]. The Rust-only `KanaClass` enum is
//! co-located with `*all-characters*` (the data that defines its
//! variants) instead of sitting in a separate file.

pub mod char_class_type;

pub mod _star_abnormal_chars_star_;
pub mod _star_all_characters_star_;
pub mod _star_basic_split_regex_star_;
pub mod _star_char_class_hash_star_;
pub mod _star_char_class_regex_mapping_star_;
