//! Rust-only sidecar within the `ichiran/dict` port module.
//! Has no Lisp counterpart.
//!
//! `KaniHintKind` is the closed two-variant enumeration of the
//! keyword tags `(:space :mod)` used by the kana-hint system. The
//! upstream uses these inline as the keyword car of plist entries —
//! both as `*hint-char-map*` keys (`dict-split.lisp:816`) and as the
//! first element of each hint pair emitted by simple-hint /
//! easy-hint macros — without a named type. CONVENTIONS §4.3 names
//! that closed tagged shape as a Rust enum.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KaniHintKind {
    Space,
    Mod,
}
