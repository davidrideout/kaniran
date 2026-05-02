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
pub mod _star_char_scanners_inner_star_;
pub mod _star_char_scanners_star_;
pub mod _star_dakuten_hash_star_;
pub mod _star_dakuten_join_star_;
pub mod _star_decimal_point_regex_star_;
pub mod _star_digit_regex_star_;
pub mod _star_hiragana_regex_star_;
pub mod _star_kanji_char_regex_star_;
pub mod _star_kanji_regex_star_;
pub mod _star_katakana_regex_star_;
pub mod _star_katakana_uniq_regex_star_;
pub mod _star_nonword_regex_star_;
pub mod _star_num_word_regex_star_;
pub mod _star_numeric_regex_star_;
pub mod _star_word_regex_star_;
