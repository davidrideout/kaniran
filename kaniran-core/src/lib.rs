//! Core library for the kaniran Rust port of ichiran.
//!
//! Layout: modules named for ichiran packages (`characters::`,
//! `dict::`, `numbers::`, ...) hold the port — one file per ported
//! Lisp symbol. Rust-only types/values with no Lisp counterpart take a
//! `kani_`/`Kani` name prefix.

pub mod characters;
pub mod conn;
pub mod core;
pub mod dict;
pub mod kanji;
pub mod numbers;
pub mod serializers;

#[cfg(test)]
mod test_support;
