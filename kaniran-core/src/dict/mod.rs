//! Port of the `ichiran/dict` package — JMdict DAO layer plus the
//! morphology / segmentation logic built on top of it.
//!
//! Initial scope (2026-05-03): row representations for the JMdict
//! kanji and kana surface forms — the leaf DAOs the counter cache
//! populator (`*counter-cache*`, wave 73) consumes as `:source` row
//! references. Further dao classes, helpers, and the populator
//! itself land in subsequent waves.

pub mod _star_counter_accepts_star_;
pub mod _star_counter_foreign_star_;
pub mod _star_counter_suffixes_star_;
pub mod kana_text_dao;
pub mod kani_suffix_kind;
pub mod kanji_text_dao;
pub mod simple_text_class;
