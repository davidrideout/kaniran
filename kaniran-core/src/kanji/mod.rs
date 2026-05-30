//! Port of the `ichiran/kanji` Lisp package — the Kanjidic2 layer.
//! Upstream `kanji.lisp` is one file; the Rust port splits it into five
//! modules mirroring the upstream's internal section order.

pub mod dao;
pub mod kanji_json;
pub mod match_json;
pub mod matching;
pub mod readings;
