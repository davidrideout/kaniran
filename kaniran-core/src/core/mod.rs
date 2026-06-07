//! Port of the bare `ichiran` package — the top-level romanization
//! and entry-point logic. Renamed to `core` in the Rust tree so it
//! does not shadow the crate root.

pub mod kani_cc_item;
pub mod kani_cc_tree;
pub mod kani_romanize_method;
pub mod romanize;
pub mod methods;
pub mod rules;
pub mod deromanize;
pub mod helpers;
