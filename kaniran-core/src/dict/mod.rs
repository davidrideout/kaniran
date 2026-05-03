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
pub mod _star_extra_counter_ids_star_;
pub mod _star_skip_counter_ids_star_;
pub mod _star_special_counters_star_;
pub mod counter_age_class;
pub mod counter_days_kun_class;
pub mod counter_days_on_class;
pub mod counter_halfhour_class;
pub mod counter_hifumi_class;
pub mod counter_months_class;
pub mod counter_people_class;
pub mod counter_text_class;
pub mod counter_tsu_class;
pub mod counter_wari_class;
pub mod kana_text_dao;
pub mod kani_suffix_kind;
pub mod kanji_text_dao;
pub mod number_text_class;
pub mod simple_text_class;
