//! Port of `ichiran/dict:*counter-foreign*` (`dict-counters.lisp:219`).
//!
//! Seqs of counter entries that should be treated as foreign — the
//! cache populator pulls katakana readings as additional sources for
//! these (not just the kanji ones). Tested for membership via
//! `(or (not kanji) (find seq *counter-foreign*))` in the populator,
//! so a counter is also implicitly foreign if it has no kanji
//! readings at all. One seq today: 1120410 (頁/ページ).

pub static COUNTER_FOREIGN: &[i32] = &[1120410];
