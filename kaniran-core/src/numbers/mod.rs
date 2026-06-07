//! Port of the `ichiran/numbers` Lisp package — Japanese number
//! parsing and rendering. The Rust-only
//! [`kani_num_class`] sidecar
//! holds the closed `NumClass` enum (the Lisp uses inline `:jd` /
//! `:p` / `:ad` keywords without a named type).

pub mod kani_num_class;
pub mod kana_form;
pub mod kanji_form;
pub mod constants;
pub mod helpers;
