//! Core library for the kaniran Rust port of ichiran.
//!
//! Layout: modules named for ichiran packages (`characters::`,
//! `dict::`, `numbers::`, ...) hold the port — one file per ported
//! Lisp symbol. Rust-only types/values with no Lisp counterpart take a
//! `kani_`/`Kani` name prefix.

pub mod characters;
pub mod conn;
pub mod core;
// The build-the-archive loaders source from Postgres; parked behind the
// `loaders` feature until they move to the standalone `kaniran-loader` crate.
#[cfg(feature = "loaders")]
pub mod custom;
pub mod dict;
pub mod kanji;
#[cfg(feature = "loaders")]
pub mod maintenance;
pub mod numbers;
