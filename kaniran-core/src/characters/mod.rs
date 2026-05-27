//! Port of the `ichiran/characters` Lisp package.
//!
//! One file per ported symbol. Rust-only sidecars (no Lisp counterpart)
//! use a `kani_<name>.rs` filename to distinguish them from
//! `_star_<name>_star_.rs` (global ports) and `<name>_<kind>.rs`
//! (typed-Lisp ports). Currently: [`kani_kana_class`].

pub mod char_class_type;
pub mod kani_char_class_bare_scanners;
pub mod kani_kana_class;

pub mod as_hiragana;
pub mod as_katakana;
pub mod basic_split;
pub mod collect_char_class;
pub mod consecutive_char_groups;
pub mod count_char_class;
pub mod dakuten_join;
pub mod destem;
pub mod geminate;
pub mod get_char_class;
pub mod join;
pub mod kanji_cross_match;
pub mod kanji_mask;
pub mod kanji_match;
pub mod kanji_prefix;
pub mod kanji_regex;
pub mod long_vowel_modifier_p;
pub mod match_diff;
pub mod mora_length;
pub mod normalize;
pub mod rendaku;
pub mod safe_subseq;
pub mod sequential_kanji_positions;
pub mod simplify_ngrams;
pub mod split_by_regex;
pub mod test_word;
pub mod to_normal_char;
pub mod unrendaku;
pub mod voice_char;

pub mod _star_abnormal_chars_star_;
pub mod _star_all_characters_star_;
pub mod _star_basic_split_regex_star_;
pub mod _star_char_class_hash_star_;
pub mod _star_char_class_regex_mapping_star_;
pub mod _star_char_scanners_inner_star_;
pub mod _star_char_scanners_star_;
pub mod _star_dakuten_hash_star_;
pub mod _star_dakuten_join_star_;
pub mod _star_decimal_point_regex_star_;
pub mod _star_digit_regex_star_;
pub mod _star_full_width_kana_star_;
pub mod _star_half_width_kana_star_;
pub mod _star_handakuten_hash_star_;
pub mod _star_hiragana_regex_star_;
pub mod _star_iteration_characters_star_;
pub mod _star_kana_characters_star_;
pub mod _star_kanji_char_regex_star_;
pub mod _star_kanji_regex_star_;
pub mod _star_katakana_regex_star_;
pub mod _star_katakana_uniq_regex_star_;
pub mod _star_modifier_characters_star_;
pub mod _star_nonword_regex_star_;
pub mod _star_normal_chars_star_;
pub mod _star_num_word_regex_star_;
pub mod _star_numeric_regex_star_;
pub mod _star_punctuation_marks_star_;
pub mod _star_sokuon_characters_star_;
pub mod _star_undakuten_hash_star_;
pub mod _star_word_regex_star_;
