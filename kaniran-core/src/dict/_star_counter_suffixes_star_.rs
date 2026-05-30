//! Port of `ichiran/dict:*counter-suffixes*` (`dict-counters.lisp:213`).
//!
//! Definition table for the three counter-trailing suffixes a counter
//! can advertise via its `:accepts` list. Each entry is
//! `(kind, kanji surface, kana, English description)`.
//!
//! Consumed by the `*counter-cache*` populator: when a counter accepts
//! a suffix, the populator synthesizes a derived cache entry by
//! appending `kanji` to the base text and `kana` to the base kana,
//! then pushes `description` onto the entry's `:suffix-descriptions`
//! list. Looked up by kind via `(cdr (assoc suf *counter-suffixes*))`
//! in the populator — linear scan over three rows.

use crate::dict::kani::SuffixKind;

pub static COUNTER_SUFFIXES: &[(SuffixKind, &str, &str, &str)] = &[
    (SuffixKind::Kan, "間", "かん", "[duration]"),
    (SuffixKind::Kango, "間後", "かんご", "[after ...]"),
    (SuffixKind::Chuu, "中", "ちゅう", "[among/out of ...]"),
];
