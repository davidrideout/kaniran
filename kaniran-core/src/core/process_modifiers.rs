//! Port of `ichiran:process-modifiers` (`romanize.lisp:17-25`).
//!
//! Folds a flat character-class list into a tree: a small-form vowel or
//! y-glide modifier wraps the item just before it, and the geminate
//! marker `:sokuon` wraps everything that follows it (recursively). All
//! other items pass through as atoms.

use super::kani_cc_item::CcItem;
use super::kani_cc_tree::CcTree;
use crate::characters::constants::MODIFIER_CHARACTERS;
use crate::characters::kani_kana_class::KanaClass;

pub fn process_modifiers(cc_list: &[CcItem]) -> Vec<CcTree> {
    let mut result: Vec<CcTree> = Vec::new();
    for (index, &cc) in cc_list.iter().enumerate() {
        match cc {
            // romanize.lisp:20-21 — (push (cons :sokuon (process-modifiers rest)) result) (loop-finish)
            CcItem::Class(KanaClass::Sokuon) => {
                result.push(CcTree::Node(
                    KanaClass::Sokuon,
                    process_modifiers(&cc_list[index + 1..]),
                ));
                break;
            }
            // romanize.lisp:22-23 — (push (list cc (pop result)) result)
            CcItem::Class(class) if is_modifier(class) => {
                let popped = result.pop().unwrap_or(CcTree::Nil);
                result.push(CcTree::Node(class, vec![popped]));
            }
            // romanize.lisp:24 — (push cc result)
            _ => result.push(CcTree::Atom(cc)),
        }
    }
    result
}

/// `(member cc *modifier-characters*)` — true for the small-form vowel,
/// y-glide, and long-vowel keyword keys of the registry.
fn is_modifier(class: KanaClass) -> bool {
    MODIFIER_CHARACTERS
        .iter()
        .any(|(modifier_class, _)| *modifier_class == class)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cls(kana: KanaClass) -> CcItem {
        CcItem::Class(kana)
    }
    fn atom(kana: KanaClass) -> CcTree {
        CcTree::Atom(CcItem::Class(kana))
    }
    fn ch(char: char) -> CcTree {
        CcTree::Atom(CcItem::Char(char))
    }
    fn node(kana: KanaClass, tail: Vec<CcTree>) -> CcTree {
        CcTree::Node(kana, tail)
    }

    #[test]
    fn process_modifiers_fixtures() {
        use KanaClass::*;
        // REPL fixtures (.103, ichiran::process-modifiers over
        // process-iteration-characters of the cited word), 2026-05-23.
        // Each row is (label, input cc-list, expected cc-tree).
        let cases: Vec<(&str, Vec<CcItem>, Vec<CcTree>)> = vec![
            // sokuon wraps the rest
            ("きっぷ", vec![cls(Ki), cls(Sokuon), cls(Pu)],
                vec![atom(Ki), node(Sokuon, vec![atom(Pu)])]),
            // modifier wraps the preceding item
            ("きゃく", vec![cls(Ki), cls(PlusYa), cls(Ku)],
                vec![node(PlusYa, vec![atom(Ki)]), atom(Ku)]),
            // long-vowel modifiers
            ("コーヒー", vec![cls(Ko), cls(LongVowel), cls(Hi), cls(LongVowel)],
                vec![node(LongVowel, vec![atom(Ko)]), node(LongVowel, vec![atom(Hi)])]),
            // leading modifier: nothing to pop, slot is nil
            ("ぁ", vec![cls(PlusA)],
                vec![node(PlusA, vec![CcTree::Nil])]),
            // nested modifiers: pop a node
            ("ゃゅ", vec![cls(PlusYa), cls(PlusYu)],
                vec![node(PlusYu, vec![node(PlusYa, vec![CcTree::Nil])])]),
            // sokuon at end wraps the empty rest
            ("っ", vec![cls(Sokuon)],
                vec![node(Sokuon, vec![])]),
            // sokuon with following mora
            ("がっこう", vec![cls(Ga), cls(Sokuon), cls(Ko), cls(U)],
                vec![atom(Ga), node(Sokuon, vec![atom(Ko), atom(U)])]),
            // modifier then sokuon
            ("しゃっくり", vec![cls(Shi), cls(PlusYa), cls(Sokuon), cls(Ku), cls(Ri)],
                vec![node(PlusYa, vec![atom(Shi)]), node(Sokuon, vec![atom(Ku), atom(Ri)])]),
            // modifier + long-vowel mix
            ("チョコレート", vec![cls(Chi), cls(PlusYo), cls(Ko), cls(Re), cls(LongVowel), cls(To)],
                vec![node(PlusYo, vec![atom(Chi)]), atom(Ko), node(LongVowel, vec![atom(Re)]), atom(To)]),
            // non-kana chars pass through as plain atoms
            ("Aと5", vec![CcItem::Char('A'), cls(To), CcItem::Char('5')],
                vec![ch('A'), atom(To), ch('5')]),
        ];
        for (label, input, expected) in &cases {
            assert_eq!(&process_modifiers(input), expected, "case={label:?}");
        }
    }
}
