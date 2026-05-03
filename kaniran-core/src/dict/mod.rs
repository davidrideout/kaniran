//! Port of the `ichiran/dict` package — JMdict DAO layer plus the
//! morphology / segmentation logic built on top of it.
//!
//! Initial scope (2026-05-03): row representations for the JMdict
//! kanji and kana surface forms — the leaf DAOs the counter cache
//! populator (`*counter-cache*`, wave 73) consumes as `:source` row
//! references. Further dao classes, helpers, and the populator
//! itself land in subsequent waves.

pub mod kana_text_dao;
pub mod kanji_text_dao;
pub mod simple_text_class;
