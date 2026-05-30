//! Port of the `ichiran/characters` Lisp package. Upstream
//! `characters.lisp` is one file; the Rust port splits it into six
//! modules mirroring the upstream's internal section order.

pub mod char_classes;
pub mod kana_class;
pub mod kanji;
pub mod normalize;
pub mod text_utils;
pub mod voicing;
