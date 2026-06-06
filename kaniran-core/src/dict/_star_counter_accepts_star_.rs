//! Port of `ichiran/dict:*counter-accepts*` (`dict-counters.lisp:217`).
//!
//! Per-seq override of which counter suffixes a JMdict entry accepts.
//! The default is "no suffixes"; entries listed here advertise the
//! given subset of [`crate::dict::kani_suffix_kind::SuffixKind`].

use crate::dict::kani_suffix_kind::SuffixKind;

pub static COUNTER_ACCEPTS: &[(i32, &[SuffixKind])] = &[
    (1194480, &[SuffixKind::Kan]),
    (1490430, &[SuffixKind::Kan]),
    (1333450, &[SuffixKind::Kan, SuffixKind::Kango]),
];
