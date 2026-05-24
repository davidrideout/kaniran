//! Rust-only sidecar within the bare-`ichiran` (`core`) port module.
//! Has no Lisp counterpart.
//!
//! `CcItem` is one element of a character-class list — the per-character
//! result of looking a glyph up in `*char-class-hash*` with the
//! default-as-self idiom: either a recognized [`KanaClass`] tag or the
//! plain [`char`] itself when the glyph is not kana. The upstream uses
//! no named type for this; it is a keyword-or-character cons element.
//! Modeled as a closed two-shape enum.

use crate::characters::kani_kana_class::KanaClass;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CcItem {
    Class(KanaClass),
    Char(char),
}
