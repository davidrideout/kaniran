//! Port of the `ichiran/dict` package — JMdict DAO layer plus the
//! morphology / segmentation logic built on top of it.
//!
//! Initial scope (2026-05-03): row representations for the JMdict
//! kanji and kana surface forms — the leaf DAOs the counter cache
//! populator (`*counter-cache*`, wave 73) consumes as `:source` row
//! references. Further dao classes, helpers, and the populator
//! itself land in subsequent waves.

pub mod _star_counter_accepts_star_;
pub mod _star_counter_cache_star_;
pub mod _star_counter_foreign_star_;
pub mod _star_counter_suffixes_star_;
pub mod _star_extra_counter_ids_star_;
pub mod _star_is_arch_cache_star_;
pub mod _star_no_conj_data_star_;
pub mod _star_skip_conj_forms_star_;
pub mod _star_skip_counter_ids_star_;
pub mod _star_special_counters_star_;
pub mod _star_suffix_cache_star_;
pub mod _star_suffix_class_star_;
pub mod _star_weak_conj_forms_star_;
pub mod counter_age_class;
pub mod counter_days_kun_class;
pub mod counter_days_on_class;
pub mod counter_halfhour_class;
pub mod counter_hifumi_class;
pub mod counter_months_class;
pub mod counter_people_class;
pub mod conj_data_prop;
pub mod conj_data_struct;
pub mod conj_prop_dao;
pub mod conj_source_reading_dao;
pub mod conjugation_dao;
pub mod counter_text_class;
pub mod counter_tsu_class;
pub mod counter_wari_class;
pub mod entry_dao;
pub mod find_word_conj_of;
pub mod find_word_seq;
pub mod get_conj_data;
pub mod get_counter_ids;
pub mod get_kana_form;
pub mod get_counter_readings;
pub mod get_counter_stags;
pub mod is_arch;
pub mod kana_text_dao;
pub mod kani_conj_form;
pub mod kani_counter_args;
pub mod kani_suffix_kind;
pub mod kanji_text_dao;
pub mod make_conj_data;
pub mod no_conj_data;
pub mod number_text_class;
pub mod sense_dao;
pub mod sense_prop_dao;
pub mod simple_text_class;
pub mod test_conj_prop;
