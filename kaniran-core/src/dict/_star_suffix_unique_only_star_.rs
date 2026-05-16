//! Port of `ichiran/dict:*suffix-unique-only*` (`dict-grammar.lisp:330`).
//!
//! Static registry consulted by [`super::match_unique::match_unique`]
//! to decide whether a `(suffix-class, matches)` pair should suppress
//! the current suffix's expansion in `find-word-suffix`. Each entry
//! tags one suffix class (lowercase string, matching the cache shape
//! per [`super::_star_suffix_cache_star_`]) with one of three
//! behaviors:
//!
//! - [`SuffixUniqueOnly::Bare`] — bare keyword entry, "unique" purely
//!   by appearing in the list. Mirrors the `(pushnew :foo
//!   *suffix-unique-only*)` entries scattered across `dict-grammar.lisp`.
//! - [`SuffixUniqueOnly::Desu`] — `(:desu . #<fn>)` cons at
//!   `dict-grammar.lisp:522-530`; closure body inlined into
//!   `match_unique` as a SQL query against `conjugation`.
//! - [`SuffixUniqueOnly::Sa`] — `(:sa . #<fn>)` cons at
//!   `dict-grammar.lisp:486-490`; closure body inlined into
//!   `match_unique` as a SQL query against `entry`.
//!
//! ## Divergences from Lisp
//!
//! The Lisp value carries SBCL closures as the cdr of the `:desu` and
//! `:sa` conses. Closures aren't representable as static Rust data, so
//! the cons-entry behavior is split: this table only tags the class;
//! [`super::match_unique::match_unique`] holds the closure bodies and
//! dispatches on the enum variant. Mirrors how every other ported
//! suffix global tags by string and the dispatcher carries the
//! behavior.
//!
//! Order in the table matches the captured runtime list (LIFO of the
//! source-line `pushnew` order — `:ii` last-pushed appears first).
//! Ordering is observable only by `find`-style FIRST-match lookup, and
//! every suffix class appears at most once, so traversal order does
//! not affect the lookup outcome.

#[derive(Debug, Clone, Copy)]
pub enum SuffixUniqueOnly {
    Bare,
    Desu,
    Sa,
}

pub static SUFFIX_UNIQUE_ONLY: &[(&str, SuffixUniqueOnly)] = &[
    ("ii", SuffixUniqueOnly::Bare),
    ("seba", SuffixUniqueOnly::Bare),
    ("meba", SuffixUniqueOnly::Bare),
    ("beba", SuffixUniqueOnly::Bare),
    ("neba", SuffixUniqueOnly::Bare),
    ("geba", SuffixUniqueOnly::Bare),
    ("keba", SuffixUniqueOnly::Bare),
    ("reba", SuffixUniqueOnly::Bare),
    ("teba", SuffixUniqueOnly::Bare),
    ("eba", SuffixUniqueOnly::Bare),
    ("dewanai", SuffixUniqueOnly::Bare),
    ("nai-n", SuffixUniqueOnly::Bare),
    ("gai", SuffixUniqueOnly::Bare),
    ("nikui", SuffixUniqueOnly::Bare),
    ("mo", SuffixUniqueOnly::Bare),
    ("desu", SuffixUniqueOnly::Desu),
    ("ra", SuffixUniqueOnly::Bare),
    ("sa", SuffixUniqueOnly::Sa),
];
