//! Port of `ichiran/dict:*counter-suffixes*` (`dict-counters.lisp:213`).
//!
//! Definition table for the three counter-trailing suffixes a counter
//! can advertise via its `:accepts` list. Each entry is
//! `(kind, kanji surface, kana, English description)`.

use crate::dict::kani_suffix_kind::SuffixKind;

pub static COUNTER_SUFFIXES: &[(SuffixKind, &str, &str, &str)] = &[
    (SuffixKind::Kan, "間", "かん", "[duration]"),
    (SuffixKind::Kango, "間後", "かんご", "[after ...]"),
    (SuffixKind::Chuu, "中", "ちゅう", "[among/out of ...]"),
];
