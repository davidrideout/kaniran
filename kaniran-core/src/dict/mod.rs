//! Port of the `ichiran/dict` package — JMdict DAO layer plus the
//! morphology / segmentation logic built on top of it.
//!
//! Initial scope (2026-05-03): row representations for the JMdict
//! kanji and kana surface forms — the leaf DAOs the counter cache
//! populator (`*counter-cache*`, wave 73) consumes as `:source` row
//! references. Further dao classes, helpers, and the populator
//! itself land in subsequent waves.

pub mod errata;
pub mod counters;
pub mod grammar;
pub mod best_path;
pub mod best_text;
pub mod calc_score;
pub mod conj_data;
pub mod dao;
pub mod find_word;
pub mod segment;
pub mod senses;
pub mod text_classes;
pub mod word_info;
pub mod load;
pub mod split;
pub mod kani;
