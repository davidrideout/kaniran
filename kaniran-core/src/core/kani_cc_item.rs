//! Rust-only sidecar within the bare-`ichiran` (`core`) port module.
//! Has no Lisp counterpart.
//!
//! `CcItem` is one element of a character-class list: either a recognized
//! [`KanaClass`] tag or the plain [`char`] itself when the glyph is not
//! kana.

use crate::characters::kani_kana_class::KanaClass;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CcItem {
    Class(KanaClass),
    Char(char),
}
