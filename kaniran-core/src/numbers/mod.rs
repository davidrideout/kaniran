//! Port of the `ichiran/numbers` Lisp package — Japanese number
//! parsing and rendering. One file per ported symbol. The Rust-only
//! [`kani_num_class`] sidecar
//! holds the closed `NumClass` enum (the Lisp uses inline `:jd` /
//! `:p` / `:ad` keywords without a named type).

pub mod constants;
pub mod kani_num_class;

pub mod group_to_kana;
pub mod not_a_number_condition;
pub mod num_sandhi;
pub mod number_to_kana;
pub mod number_to_kanji;
pub mod parse_number;
pub mod parse_number_star_;

pub mod _star_char_number_class_hash_star_;
