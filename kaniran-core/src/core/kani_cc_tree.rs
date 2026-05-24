//! Rust-only sidecar within the bare-`ichiran` (`core`) port module.
//! Has no Lisp counterpart.
//!
//! `CcTree` is one node of the tree `process-modifiers` builds and
//! `romanize-core` / `leftmost-atom` walk. The upstream uses bare cons
//! cells: a node is either an atom (a keyword or a character), `nil`
//! (the empty slot left when a modifier has no preceding item to wrap),
//! or a list whose head is a modifier / sokuon keyword and whose tail is
//! itself a list of nodes — `(:+ya <child>)` for a modifier or
//! `(:sokuon . <rest>)` for the geminate marker. The three Rust variants
//! mirror those three cons shapes; `Node`'s head is always a [`KanaClass`]
//! because `process-modifiers` only ever conses a modifier or `:sokuon`
//! keyword onto a tail.

use super::kani_cc_item::CcItem;
use crate::characters::kani_kana_class::KanaClass;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CcTree {
    Nil,
    Atom(CcItem),
    Node(KanaClass, Vec<CcTree>),
}
