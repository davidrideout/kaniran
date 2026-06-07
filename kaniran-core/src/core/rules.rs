use super::deromanize::{possible_long_vowel_p, KanaRepresentation, RmapItem};
use super::kani_cc_item::CcItem;
use super::kani_cc_tree::CcTree;
use super::methods::RomanizationMethod;
use super::romanize::{leftmost_atom, romanize_core};
use crate::characters::constants::MODIFIER_CHARACTERS;
use crate::characters::kani_kana_class::KanaClass;
use crate::characters::voicing::voice_char;

/// Port of `ichiran:r-apply` (gf — `romanize.lisp:44-55`, `69-77`, `106-130`).
///
/// Applies a modifier to its wrapped character-class subtree: `:sokuon`
/// doubles the lead consonant (or prefixes `t` before `chi` under
/// hepburn), `:long-vowel` returns the inner romanization, and the
/// small-form vowel / y-glide modifiers consult the kana-table — with
/// hepburn's `sha`/`cha`/`ja` family overriding the y-glides over
/// `shi`/`chi`/`ji`.
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

/// Port of `ichiran:r-base` (gf — `romanize.lisp:39-42`, `66-67`).
///
/// Romanizes one atomic mora class by looking it up in the method's
/// kana-table, falling back to the downcased class name.
pub fn r_base(method: RomanizationMethod<'_>, item: KanaClass) -> String {
    match method.kana_table().get(&item) {
        // romanize.lisp:66-67 (r-base generic-romanization)
        Some(latin) => (*latin).to_string(),
        // romanize.lisp:41-42 (r-base default — string-downcase)
        None => item.lisp_name().to_ascii_lowercase(),
    }
}

/// Port of `ichiran:r-simplify` (gf — `romanize.lisp:57-59`, `132-199`).
///
/// Post-processes a romanized string per the method's orthography
/// (apostrophe handling, long-vowel folding, the traditional `n-`/`m`
/// rules); the default is the identity.
pub fn r_simplify(method: RomanizationMethod<'_>, str: &str) -> String {
    match method {
        RomanizationMethod::GenericHepburn(method) => method.r_simplify(str),
        RomanizationMethod::SimplifiedHepburn(method) => method.r_simplify(str),
        RomanizationMethod::TraditionalHepburn(method) => method.r_simplify(str),
        RomanizationMethod::ModifiedHepburn(method) => method.r_simplify(str),
        RomanizationMethod::KunreiSiki(method) => method.r_simplify(str),
    }
}

/// Port of `ichiran:r-special` (gf — `romanize.lisp:210-215`).
///
/// Romanizes the standalone glyphs that carry no mora class: a lone small
/// tsu (`っ` -> `!`) and the long-vowel bar (`ー` -> `~`).
pub fn r_special(method: RomanizationMethod<'_>, word: &str) -> Option<String> {
    let _ = method;
    // romanize.lisp:213-215 (r-special or — default method)
    match word {
        "っ" => Some("!".to_string()),
        "ー" => Some("~".to_string()),
        _ => None,
    }
}

/// Port of `ichiran:process-iteration-characters` (`romanize.lisp:9-15`).
///
/// Expands iteration markers in a character-class list: `Iter` (ゝヽ)
/// repeats the previous item, `IterV` (ゞヾ) emits its voiced form. A run
/// of markers all expand to the same source; a marker at the start (no
/// previous item) drops out silently.
pub fn process_iteration_characters(cc_list: &[CcItem]) -> Vec<CcItem> {
    let mut out = Vec::with_capacity(cc_list.len());
    let mut prev: Option<CcItem> = None;
    for &cc in cc_list {
        match cc {
            CcItem::Class(KanaClass::Iter) => {
                if let Some(p) = prev {
                    out.push(p);
                }
            }
            CcItem::Class(KanaClass::IterV) => {
                if let Some(p) = prev {
                    out.push(match p {
                        CcItem::Class(k) => CcItem::Class(voice_char(k)),
                        CcItem::Char(_) => p,
                    });
                }
            }
            other => {
                out.push(other);
                prev = Some(other);
            }
        }
    }
    out
}

/// Port of `ichiran:process-modifiers` (`romanize.lisp:17-25`).
///
/// Folds a flat character-class list into a tree: a small-form vowel or
/// y-glide modifier wraps the item just before it, and the geminate
/// marker `:sokuon` wraps everything that follows it (recursively). All
/// other items pass through as atoms.
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

/// Port of `ichiran:apply-rmap-item` (`deromanize.lisp:37`).
///
/// Builds the [`KanaRepresentation`] for applying one romaji rule
/// `rmi` to input `s`, appending a trailing `う?` to the pattern when
/// the consumed romaji could be a long vowel.
pub fn apply_rmap_item(s: &str, rmi: &RmapItem) -> KanaRepresentation {
    let kana = &rmi.kana;
    KanaRepresentation {
        canonical: kana.clone(),
        pattern: if possible_long_vowel_p(&rmi.text).is_some() {
            format!("{kana}う?")
        } else {
            kana.clone()
        },
        rest: format!(
            "{}{}",
            rmi.next.as_deref().unwrap_or(""),
            s.chars().skip(rmi.text.chars().count()).collect::<String>()
        ),
        branch: 0,
    }
}

#[cfg(test)]
mod tests;
