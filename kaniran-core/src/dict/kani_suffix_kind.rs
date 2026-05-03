//! Rust-only sidecar (CONVENTIONS §2): closed enum for the counter
//! suffix tags. The Lisp uses these inline as `:kan`, `:kango`, `:chuu`
//! without a named type; verified exhaustive by a REPL sweep of every
//! `:accepts` value carried in the populated `*counter-cache*`.
//!
//! Used as both the key column of
//! [`crate::dict::_star_counter_suffixes_star_::COUNTER_SUFFIXES`] (the
//! suffix definition table) and the value-list element type of
//! [`crate::dict::_star_counter_accepts_star_::COUNTER_ACCEPTS`] (the
//! per-seq override of which suffixes a counter takes).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SuffixKind {
    Kan,
    Kango,
    Chuu,
}
