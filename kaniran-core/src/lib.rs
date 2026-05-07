//! Core library for the kaniran Rust port of ichiran.
//!
//! Layout:
//! - [`kani`] — kaniran-specific infrastructure (fixture replay, name
//!   translation). Original code, not ported from ichiran.
//! - Sibling modules named for ichiran packages (`characters::`,
//!   `dict::`, `numbers::`, ...) hold the actual port. One file per
//!   ported Lisp symbol; see [`kani::naming`] for the mapping rule.

pub mod kani;

pub mod characters;
pub mod conn;
pub mod core;
pub mod dict;
pub mod kanji;
pub mod maintenance;
pub mod numbers;
