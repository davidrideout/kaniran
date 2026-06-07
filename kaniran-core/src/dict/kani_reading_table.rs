//! Rust-only sidecar: 2-variant tag for picking between the
//! `kanji_text` and `kana_text` reading tables at runtime.
//!
//! Example values:
//! - `KaniReadingTable::Kanji` → table `"kanji_text"` (DAO
//!   [`crate::dict::dao::KanjiText`]).
//! - `KaniReadingTable::Kana` → table `"kana_text"` (DAO
//!   [`crate::dict::dao::KanaText`]).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KaniReadingTable {
    Kanji,
    Kana,
}

impl KaniReadingTable {
    pub fn table_name(self) -> &'static str {
        match self {
            KaniReadingTable::Kanji => "kanji_text",
            KaniReadingTable::Kana => "kana_text",
        }
    }
}
