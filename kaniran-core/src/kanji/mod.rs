//! Port of the `ichiran/kanji` package — the Kanjidic2 layer.
//!
//! Initial scope (2026-05-07): row representations for the four
//! kanjidic DAO classes (`kanji`, `reading`, `okurigana`, `meaning`).
//! The XML loader and lookup helpers built on top of them land in
//! subsequent waves.

pub mod _star_kanjidic_path_star_;
pub mod _star_reading_cache_star_;
pub mod calculate_perc;
pub mod first_node_text;
pub mod get_normal_readings;
pub mod get_original_reading;
pub mod get_reading_alternatives;
pub mod get_reading_stats;
pub mod get_readings;
pub mod get_readings_cache;
pub mod init_tables;
pub mod kani_kanji_reading;
pub mod kanji_dao;
pub mod kanji_info_json;
pub mod kanji_reading_json;
pub mod kanji_word_stats;
pub mod load_kanji;
pub mod load_kanji_stats;
pub mod load_kanjidic;
pub mod load_readings;
pub mod make_rmap;
pub mod match_readings;
pub mod match_readings_json;
pub mod match_readings_star_;
pub mod meaning_dao;
pub mod okurigana_dao;
pub mod process_match_json;
pub mod query_kanji_json_macro;
pub mod reading_dao;
pub mod reading_info_json;
pub mod to_json;
