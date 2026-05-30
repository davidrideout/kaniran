//! Port of `ichiran/dict-grammar.lisp` — the suffix / abbreviation /
//! synergy / penalty / segfilter rule systems plus the find-word
//! lookups and filter predicates they share.
//!
//! Nine sections mirror the upstream's internal structure:
//!
//! - [`find_word`] — `find-word-with-{conj-prop, conj-type, pos, suffix}`,
//!   `find-word-conj-of`, `find-word-seq`, `pair-words-by-conj`,
//!   `or-as-hiragana`, plus the kana-form selectors.
//! - [`suffix_init`] — `*suffix-{list, cache, class, description,
//!   unique-only}*` and the `init-suffixes-thread` builder.
//! - [`suffix_rules`] — `def-simple-suffix` + every callsite.
//! - [`abbr`] — `def-abbr-suffix` + 15 callsites.
//! - [`suffix_lookup`] — `get-suffix-map`, `get-suffixes`, `match-unique`,
//!   `find-word-suffix`.
//! - [`synergy`] — `synergy` struct, `*synergy-list*`, `def-generic-synergy`
//!   + 17 bodies, `make-segment-list-from`, `get-synergies`.
//! - [`filter`] — seq-set / pos / noun / conjugation / compound-end /
//!   short-kana predicates plus `*noun-particles*` and `classify`.
//! - [`penalty`] — `*penalty-list*`, `def-generic-penalty`, `penalty-short`,
//!   `penalty-semi-final`, `get-penalties`.
//! - [`segfilter`] — `*segfilter-list*`, `def-segfilter-must-follow`,
//!   `*aux-verbs*` / `*honorifics*`, 16 callsites, `apply-segfilters`.
//!
//! Each section file wraps its original per-symbol source bodies in
//! `mod {symbol}_inner { ... }` blocks and re-exports them via
//! `pub use {symbol}_inner::*;`. Each source's imports stay scoped to
//! its own body — no name collisions, no cross-source dedup needed,
//! while `dict::grammar::{section}::{symbol}` remains the public path.

pub mod abbr;
pub mod filter;
pub mod find_word;
pub mod penalty;
pub mod segfilter;
pub mod suffix_init;
pub mod suffix_lookup;
pub mod suffix_rules;
pub mod synergy;
