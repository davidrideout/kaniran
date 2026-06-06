//! Port of `ichiran/dict:*kana-hint-mod*` (`dict-split.lisp:813`).
//!
//! Sentinel character marking a kana-particle boundary that the
//! romanizer should rewrite (`は → wa`, `へ → e`, …).

pub const KANA_HINT_MOD: char = '\u{200c}';
