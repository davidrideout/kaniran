//! Rust-only sidecar within the `ichiran/dict` port module.
//! Has no Lisp counterpart.
//!
//! `KaniMatchPart` is the closed two-variant enumeration of an
//! element in the heterogeneous "match" list consumed by
//! [`super::translate_hint_position`] and
//! [`super::translate_hints`]. The upstream feeds those functions
//! the raw output of `match-diff` (a list whose atoms are equal
//! substrings and whose conses are `(s1 s2)` differing-pair
//! substrings — `characters.lisp:326`) and the raw output of
//! `match-readings` (an analogous list of bare strings or
//! `(kanji-char reading-text)` pairs — `kanji.lisp:296`); CL's
//! `(if (atom part) ... ...)` dispatch picks the branch.
//!
//! The Lisp functions only ever read the `length` of each substring
//! — never the characters themselves — so the Rust port carries
//! pre-computed character counts in the variants rather than the
//! original substrings. Callers converting `MatchSegment` /
//! `MatchedSegment` output drop the strings at conversion time;
//! observable behavior is identical.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KaniMatchPart {
    Atom(usize),
    Pair(usize, usize),
}
