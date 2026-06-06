//! Rust-only sidecar within the `ichiran/dict` port module.
//! Has no Lisp counterpart.
//!
//! `KaniMatchPart` is the closed two-variant enumeration of an
//! element in the heterogeneous "match" list consumed by
//! [`super::translate_hint_position`] and [`super::translate_hints`]:
//! an `Atom` (equal substring) or a `Pair` (differing-pair). Only the
//! substring lengths matter, so the variants carry character counts
//! rather than the substrings.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KaniMatchPart {
    Atom(usize),
    Pair(usize, usize),
}
