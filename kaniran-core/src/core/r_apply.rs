//! Port of `ichiran:r-apply` (gf — `romanize.lisp:44-55`, `69-77`, `106-130`).
//!
//! Applies a modifier to its wrapped character-class subtree. Dispatch is
//! on the modifier value and the method class: `:sokuon` doubles the lead
//! consonant (or prefixes `t` before `chi` under hepburn), `:long-vowel`
//! returns the inner romanization, and the small-form vowel / y-glide
//! modifiers consult the kana-table — with hepburn's `sha`/`cha`/`ja`
//! family overriding the y-glides over `shi`/`chi`/`ji`.

use super::generic_romanization_class::RomanizationMethod;
use super::kani_cc_item::CcItem;
use super::kani_cc_tree::CcTree;
use super::leftmost_atom::leftmost_atom;
use super::romanize_core::romanize_core;
use crate::characters::kani_kana_class::KanaClass;

pub fn r_apply(modifier: KanaClass, method: RomanizationMethod<'_>, cc_tree: &[CcTree]) -> String {
    // The hepburn-specialized methods (sokuon-before-chi, :+ya/:+yu/:+yo)
    // dispatch on generic-hepburn and its subclasses; kunrei-siki does not.
    let hepburn = matches!(
        method,
        RomanizationMethod::GenericHepburn(_)
            | RomanizationMethod::SimplifiedHepburn(_)
            | RomanizationMethod::TraditionalHepburn(_)
            | RomanizationMethod::ModifiedHepburn(_)
    );
    match modifier {
        KanaClass::Sokuon => {
            // romanize.lisp:106-109 (r-apply :sokuon generic-hepburn)
            if hepburn && leftmost_atom(cc_tree) == Some(CcItem::Class(KanaClass::Chi)) {
                format!("t{}", romanize_core(method, cc_tree))
            } else {
                // romanize.lisp:46-51 (r-apply :sokuon T) — double a Basic-Latin lead
                let inner = romanize_core(method, cc_tree);
                match inner.chars().next() {
                    Some(first) if (first as u32) <= 0x7F => format!("{first}{inner}"),
                    _ => inner,
                }
            }
        }
        // romanize.lisp:52-53 (r-apply :long-vowel T)
        KanaClass::LongVowel => romanize_core(method, cc_tree),
        _ => {
            // romanize.lisp:111-130 (r-apply :+ya/:+yu/:+yo generic-hepburn)
            if hepburn {
                if let Some(special) = hepburn_yoon(modifier, cc_tree.first()) {
                    return special.to_string();
                }
            }
            // romanize.lisp:69-77 (r-apply symbol generic-romanization)
            let kana_table = method.kana_table();
            match kana_table.get(&modifier).copied() {
                Some(yoon) => match cc_tree.first() {
                    Some(CcTree::Atom(CcItem::Class(KanaClass::U))) => format!("w{yoon}"),
                    // romanize.lisp:74 — (gethash (car cc-tree) (kana-table method));
                    // a table miss prints NIL via ~a (unreachable: a/i/e/o are
                    // always keys whenever a modifier yoon was found).
                    Some(CcTree::Atom(CcItem::Class(
                        head @ (KanaClass::A | KanaClass::I | KanaClass::E | KanaClass::O),
                    ))) => format!("{}{}", kana_table.get(head).copied().unwrap_or("NIL"), yoon),
                    _ => {
                        let inner = romanize_core(method, cc_tree);
                        let keep = inner.chars().count().saturating_sub(1);
                        let trimmed: String = inner.chars().take(keep).collect();
                        format!("{trimmed}{yoon}")
                    }
                },
                // romanize.lisp:54-55 (r-apply symbol T — string-downcase)
                None => format!(
                    "{}{}",
                    romanize_core(method, cc_tree),
                    modifier.lisp_name().to_ascii_lowercase()
                ),
            }
        }
    }
}

/// romanize.lisp:111-130 — generic-hepburn `:+ya`/`:+yu`/`:+yo`. Returns
/// `None` for the `(t (call-next-method))` arm (any other lead, or a
/// non-mora lead, falls through to the generic-romanization method).
fn hepburn_yoon(modifier: KanaClass, head: Option<&CcTree>) -> Option<&'static str> {
    use KanaClass::*;
    let head = match head {
        Some(CcTree::Atom(CcItem::Class(class))) => *class,
        _ => return None,
    };
    match (modifier, head) {
        (PlusYa, Shi) => Some("sha"),
        (PlusYa, Chi) => Some("cha"),
        (PlusYa, Ji | Dji) => Some("ja"),
        (PlusYu, Shi) => Some("shu"),
        (PlusYu, Chi) => Some("chu"),
        (PlusYu, Ji | Dji) => Some("ju"),
        (PlusYo, Shi) => Some("sho"),
        (PlusYo, Chi) => Some("cho"),
        (PlusYo, Ji | Dji) => Some("jo"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::generic_hepburn_class::GenericHepburn;
    use crate::core::generic_romanization_class::GenericRomanization;
    use crate::core::kunrei_siki_class::KunreiSiki;

    fn atom(class: KanaClass) -> CcTree {
        CcTree::Atom(CcItem::Class(class))
    }
    fn chr(character: char) -> CcTree {
        CcTree::Atom(CcItem::Char(character))
    }

    #[test]
    fn r_apply_fixtures() {
        use KanaClass::*;
        let hepburn = GenericHepburn::new();
        let kunrei = KunreiSiki::new();
        // An emptied generic-romanization reaches the symbol-T downcase
        // fallback; with a populated table that branch is unreachable.
        let mut bare = GenericHepburn::new();
        bare.0 = GenericRomanization::new();
        let h = RomanizationMethod::GenericHepburn(&hepburn);
        let k = RomanizationMethod::KunreiSiki(&kunrei);
        let b = RomanizationMethod::GenericHepburn(&bare);
        // REPL fixtures (.103, ichiran::r-apply), 2026-05-24.
        // (label, modifier, method, cc-tree, expected).
        let cases: &[(&str, KanaClass, RomanizationMethod, Vec<CcTree>, &str)] = &[
            // sokuon: hepburn chi -> "t", else double a Basic-Latin lead
            ("sokuon hepburn chi", Sokuon, h, vec![atom(Chi)], "tchi"),
            ("sokuon hepburn pu", Sokuon, h, vec![atom(Pu)], "ppu"),
            ("sokuon hepburn empty", Sokuon, h, vec![], ""),
            ("sokuon hepburn cyrillic", Sokuon, h, vec![chr('я')], "я"),
            ("sokuon kunrei chi", Sokuon, k, vec![atom(Chi)], "tti"),
            ("sokuon kunrei pu", Sokuon, k, vec![atom(Pu)], "ppu"),
            // long-vowel: inner romanization unchanged
            ("long-vowel hepburn ko", LongVowel, h, vec![atom(Ko)], "ko"),
            // hepburn y-glide overrides over shi/chi/ji/dji
            ("+ya hepburn shi", PlusYa, h, vec![atom(Shi)], "sha"),
            ("+ya hepburn chi", PlusYa, h, vec![atom(Chi)], "cha"),
            ("+ya hepburn ji", PlusYa, h, vec![atom(Ji)], "ja"),
            ("+ya hepburn dji", PlusYa, h, vec![atom(Dji)], "ja"),
            ("+ya hepburn ki", PlusYa, h, vec![atom(Ki)], "kya"),
            ("+yu hepburn shi", PlusYu, h, vec![atom(Shi)], "shu"),
            ("+yu hepburn ki", PlusYu, h, vec![atom(Ki)], "kyu"),
            ("+yo hepburn chi", PlusYo, h, vec![atom(Chi)], "cho"),
            ("+yo hepburn ki", PlusYo, h, vec![atom(Ki)], "kyo"),
            // kunrei has no y-glide override -> generic-romanization path
            ("+ya kunrei shi", PlusYa, k, vec![atom(Shi)], "sya"),
            ("+ya kunrei ki", PlusYa, k, vec![atom(Ki)], "kya"),
            // generic-romanization symbol cases: :u, :a/:i/:e/:o, default
            ("+a hepburn u", PlusA, h, vec![atom(U)], "wa"),
            ("+a hepburn a", PlusA, h, vec![atom(A)], "aa"),
            ("+i hepburn i", PlusI, h, vec![atom(I)], "ii"),
            ("+wa hepburn ku", PlusWa, h, vec![atom(Ku)], "kwa"),
            ("+a hepburn ki", PlusA, h, vec![atom(Ki)], "ka"),
            // symbol-T fallback: modifier missing from the table -> downcase
            ("+ya bare ki", PlusYa, b, vec![atom(Ki)], "ki+ya"),
        ];
        for (label, modifier, method, cc_tree, expected) in cases {
            assert_eq!(&r_apply(*modifier, *method, cc_tree), expected, "case={label}");
        }
    }
}
