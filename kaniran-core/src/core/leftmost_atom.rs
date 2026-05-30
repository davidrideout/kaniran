//! Port of `ichiran:leftmost-atom` (`romanize.lisp:27-29`).
//!
//! Descends the leftmost branch of a character-class tree and returns
//! the first atom it reaches. `Option<CcItem>` collapses the upstream's
//! atom-or-nil return: `nil` (empty list, or a nil leaf) becomes `None`;
//! a keyword or character leaf becomes `Some`.

use super::kani_cc_item::CcItem;
use super::kani_cc_tree::CcTree;

pub fn leftmost_atom(cc_list: &[CcTree]) -> Option<CcItem> {
    match cc_list.first() {
        // (car nil) is nil and (atom nil) is true -> return nil.
        None | Some(CcTree::Nil) => None,
        Some(CcTree::Atom(item)) => Some(*item),
        Some(CcTree::Node(_, rest)) => leftmost_atom(rest),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characters::kana_class::KanaClass;

    fn atom(kana: KanaClass) -> CcTree {
        CcTree::Atom(CcItem::Class(kana))
    }
    fn node(kana: KanaClass, tail: Vec<CcTree>) -> CcTree {
        CcTree::Node(kana, tail)
    }

    #[test]
    fn leftmost_atom_fixtures() {
        use KanaClass::*;
        // REPL fixtures (.103, ichiran::leftmost-atom), 2026-05-23.
        // Each row is (label, input cc-tree, expected leftmost atom).
        let cases: Vec<(&str, Vec<CcTree>, Option<CcItem>)> = vec![
            // first element is already an atom
            ("(:TA)", vec![atom(Ta)], Some(CcItem::Class(Ta))),
            // flat list returns the head
            ("(:SO :U :SHI)", vec![atom(So), atom(U), atom(Shi)], Some(CcItem::Class(So))),
            // descends into a modifier node
            ("((:+YA :CHI))", vec![node(PlusYa, vec![atom(Chi)])], Some(CcItem::Class(Chi))),
            // descends through a sokuon node
            ("((:SOKUON (:+YA :CHI)))",
                vec![node(Sokuon, vec![node(PlusYa, vec![atom(Chi)])])],
                Some(CcItem::Class(Chi))),
            // descends through nested modifiers
            ("((:+YU (:+YA :CHI)))",
                vec![node(PlusYu, vec![node(PlusYa, vec![atom(Chi)])])],
                Some(CcItem::Class(Chi))),
            // empty list is nil
            ("NIL", vec![], None),
            // a nil leaf is the leftmost atom
            ("((:+YA NIL))", vec![node(PlusYa, vec![CcTree::Nil])], None),
            // a char leaf
            ("(#\\a)", vec![CcTree::Atom(CcItem::Char('a'))], Some(CcItem::Char('a'))),
        ];
        for (label, input, expected) in &cases {
            assert_eq!(&leftmost_atom(input), expected, "case={label}");
        }
    }
}
