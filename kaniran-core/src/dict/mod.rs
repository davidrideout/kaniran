//! Port of the `ichiran/dict` package — JMdict DAO layer plus the
//! morphology / segmentation logic built on top of it.

pub mod kani_conj_form;
pub mod kani_lite_segment;
pub mod kani_lite_segment_list;
pub mod kani_lite_top_array;
pub mod kani_lite_top_array_item;
pub mod kani_match_part;
pub mod kani_reading_table;
pub mod kani_word;
pub mod errata;
pub mod counters;
pub mod load;
pub mod grammar;
pub mod split;
pub mod dao;
pub mod accessors;
pub mod conj;
pub mod readings;
pub mod text_classes;
pub mod scoring;
pub mod path;
pub mod word_info;
pub mod senses;
pub mod word_info_str;
pub mod find_word_info;
