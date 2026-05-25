//! Port of the bare `ichiran` package — the top-level romanization
//! and entry-point logic. Renamed to `core` in the Rust tree so it
//! does not shadow the crate root; see [`crate::kani::naming`].

pub mod _star_default_romanization_method_star_;
pub mod _star_hepburn_basic_star_;
pub mod _star_hepburn_kana_table_star_;
pub mod _star_hepburn_simple_star_;
pub mod _star_hepburn_traditional_star_;
pub mod _star_kunrei_siki_kana_table_star_;
pub mod generic_hepburn_class;
pub mod generic_romanization_class;
pub mod get_character_classes;
pub mod join_parts;
pub mod kana_representation_struct;
pub mod kani_cc_item;
pub mod kani_cc_tree;
pub mod kani_romanize_method;
pub mod kunrei_siki_class;
pub mod leftmost_atom;
pub mod process_iteration_characters;
pub mod process_modifiers;
pub mod r_apply;
pub mod r_base;
pub mod r_simplify;
pub mod r_special;
pub mod rmap_item_struct;
pub mod romanize;
pub mod romanize_core;
pub mod romanize_list;
pub mod romanize_star_;
pub mod romanize_word;
pub mod romanize_word_geo;
pub mod romanize_word_info;
pub mod simplified_hepburn_class;
pub mod traditional_hepburn_class;
