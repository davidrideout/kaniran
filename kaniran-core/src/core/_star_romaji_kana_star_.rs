//! Port of `ichiran:*romaji-kana*` (`deromanize.lisp:7`, `csv-hash *romaji-kana*`).
//!
//! Maps a romaji prefix to its [`RmapItem`] kana rule.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::load_romaji_kana::load_romaji_kana;
use super::rmap_item_struct::RmapItem;

pub fn romaji_kana() -> &'static HashMap<String, RmapItem> {
    static ROMAJI_KANA: OnceLock<HashMap<String, RmapItem>> = OnceLock::new();
    ROMAJI_KANA.get_or_init(load_romaji_kana)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `OnceLock` builder wiring runs `load_romaji_kana` (292
    /// entries per the .103 REPL).
    #[test]
    fn builds_once() {
        assert_eq!(romaji_kana().len(), 292);
    }
}
