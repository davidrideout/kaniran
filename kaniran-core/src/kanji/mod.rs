//! Port of the `ichiran/kanji` package — the Kanjidic2 layer.
//!
//! Initial scope (2026-05-07): row representations for the four
//! kanjidic DAO classes (`kanji`, `reading`, `okurigana`, `meaning`).
//! The XML loader and lookup helpers built on top of them land in
//! subsequent waves.

pub mod kani_kanji_reading;
pub mod dao;
// Kanjidic XML loader — sources into Postgres; parked behind `loaders`
// until it moves to the standalone `kaniran-loader` crate.
#[cfg(feature = "loaders")]
pub mod loaders;
pub mod readings;
pub mod matching;
pub mod stats;
pub mod json;
pub mod helpers;
