//! Port of the bare `ichiran` package — the top-level romanization
//! and entry-point logic. Renamed to `core` in the Rust tree so it
//! does not shadow the crate root; see [`crate::kani::naming`].

pub mod kana_representation_struct;
pub mod process_iteration_characters;
pub mod rmap_item_struct;
