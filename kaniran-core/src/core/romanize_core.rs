//! Port of `ichiran:romanize-core` (`romanize.lisp:31-37`).
//!
//! Walks a character-class tree, concatenating each node's romanization:
//! `nil` drops out, a raw character passes through unchanged, a mora class
//! goes through `r-base`, and a modifier node through `r-apply`.

use super::generic_romanization_class::RomanizationMethod;
use super::kani_cc_item::CcItem;
use super::kani_cc_tree::CcTree;
use super::r_apply::r_apply;
use super::r_base::r_base;

pub fn romanize_core(method: RomanizationMethod<'_>, cc_tree: &[CcTree]) -> String {
    let mut out = String::new();
    for item in cc_tree {
        match item {
            // (null item)
            CcTree::Nil => {}
            // (characterp item) (princ item out)
            CcTree::Atom(CcItem::Char(character)) => out.push(*character),
            // (atom item) (princ (r-base method item) out)
            CcTree::Atom(CcItem::Class(class)) => out.push_str(&r_base(method, *class)),
            // (listp item) (princ (r-apply (car item) method (cdr item)) out)
            CcTree::Node(modifier, tail) => out.push_str(&r_apply(*modifier, method, tail)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characters::kani_kana_class::KanaClass;
    use crate::core::generic_hepburn_class::GenericHepburn;

    #[test]
    fn romanize_core_walks_every_node_shape() {
        use KanaClass::*;
        let hepburn = GenericHepburn::new();
        let method = RomanizationMethod::GenericHepburn(&hepburn);
        // REPL (.103, (romanize-core *hepburn-basic* '(:ko nil :n #\x (:+ya :ki))))
        // => "kon'xkya", 2026-05-24 — exercises mora class, nil skip, raw
        // character passthrough, and a modifier node in one tree.
        let cc_tree = vec![
            CcTree::Atom(CcItem::Class(Ko)),
            CcTree::Nil,
            CcTree::Atom(CcItem::Class(N)),
            CcTree::Atom(CcItem::Char('x')),
            CcTree::Node(PlusYa, vec![CcTree::Atom(CcItem::Class(Ki))]),
        ];
        assert_eq!(romanize_core(method, &cc_tree), "kon'xkya");
    }
}
