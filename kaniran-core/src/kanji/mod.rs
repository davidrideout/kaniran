//! Port of the `ichiran/kanji` package — the Kanjidic2 layer.
//!
//! Initial scope (2026-05-07): row representations for the four
//! kanjidic DAO classes (`kanji`, `reading`, `okurigana`, `meaning`).
//! The XML loader and lookup helpers built on top of them land in
//! subsequent waves.

pub mod kanji_dao;
pub mod meaning_dao;
pub mod okurigana_dao;
pub mod reading_dao;
