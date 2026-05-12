//! Port of `ichiran/dict:*kana-hint-mod*` (`dict-split.lisp:813`).
//!
//! Sentinel character marking a kana-particle boundary that the
//! romanizer should rewrite (`は → wa`, `へ → e`, …). Inserted by the
//! hint system and consumed by [`super::_star_hint_simplify_map_star_`]
//! during romanization.

pub const KANA_HINT_MOD: char = '\u{200c}';
