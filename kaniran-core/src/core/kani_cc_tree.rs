//! Rust-only sidecar within the bare-`ichiran` (`core`) port module.
//! Has no Lisp counterpart.
//!
//! `CcTree` is one node of the tree `process-modifiers` builds and
//! `romanize-core` / `leftmost-atom` walk: an empty slot (`Nil`), an atom
//! (keyword or character), or a node with a [`KanaClass`] head (modifier
//! or sokuon) and a list of child nodes.

use super::kani_cc_item::CcItem;
use crate::characters::kani_kana_class::KanaClass;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CcTree {
    Nil,
    Atom(CcItem),
    Node(KanaClass, Vec<CcTree>),
}
