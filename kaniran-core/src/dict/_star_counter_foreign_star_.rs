//! Port of `ichiran/dict:*counter-foreign*` (`dict-counters.lisp:219`).
//!
//! Seqs of counter entries treated as foreign, so their katakana
//! readings are pulled as additional sources. One seq today: 1120410
//! (頁/ページ).

pub static COUNTER_FOREIGN: &[i32] = &[1120410];
