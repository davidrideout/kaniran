//! The rewrite-rule machinery: classification, iteration/modifier
//! folding, the `r-*` family, and `romanize-core`. From
//! `romanize.lisp:5-77, 106-215, 235-246`.
//!
//! `romanize_core` lives here (not in `romanize.rs`) because it and
//! `r_apply` are mutually recursive.

use unicode_properties::{GeneralCategory, GeneralCategoryGroup, UnicodeGeneralCategory};

use super::methods::{CcItem, CcTree, RomanizationMethod};
use crate::characters::kana_class::{char_class_hash, KanaClass, MODIFIER_CHARACTERS};
use crate::characters::voicing::voice_char;

/// `get-character-classes` (`romanize.lisp:5-7`). One [`CcItem`] per
/// char of `word`: `Class(k)` when the glyph is in `*char-class-hash*`,
/// `Char(c)` otherwise.
pub fn get_character_classes(word: &str) -> Vec<CcItem> {
    word.chars()
        .map(|char| match char_class_hash().get(&char) {
            Some(&class) => CcItem::Class(class),
            None => CcItem::Char(char),
        })
        .collect()
}

/// `process-iteration-characters` (`romanize.lisp:9-15`). Expand `Iter`
/// (ゝヽ) into a repeat of the previous item; `IterV` (ゞヾ) into the
/// voiced previous item. The "previous" slot only updates on a
/// non-iteration item, so a run of markers all expand from the same
/// source. An iteration marker at the start drops silently.
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
                        // voice-char hash misses on a raw char and returns
                        // input unchanged.
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

/// `process-modifiers` (`romanize.lisp:17-25`). Fold a flat list into a
/// tree: a small-form vowel or y-glide modifier wraps the preceding
/// item, `:sokuon` wraps everything that follows it (recursively).
/// Upstream uses `push` + `nreverse`; the Rust keeps forward order and
/// pushes/pops the back, no reversal needed.
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

fn is_modifier(class: KanaClass) -> bool {
    MODIFIER_CHARACTERS
        .iter()
        .any(|(modifier_class, _)| *modifier_class == class)
}

/// `leftmost-atom` (`romanize.lisp:27-29`). First atom on the leftmost
/// branch of a tree. `None` for an empty list or a nil leaf.
pub fn leftmost_atom(cc_list: &[CcTree]) -> Option<CcItem> {
    match cc_list.first() {
        None | Some(CcTree::Nil) => None,
        Some(CcTree::Atom(item)) => Some(*item),
        Some(CcTree::Node(_, rest)) => leftmost_atom(rest),
    }
}

/// `r-base` (gf — `romanize.lisp:39-42, 66-67`). Romanize one atomic
/// mora class. The `(generic-romanization item)` method always applies
/// (every method instance is one); it looks the class up in
/// `kana-table`. The default `(method item)` downcases the printed
/// keyword name when the table misses.
pub fn r_base(method: RomanizationMethod<'_>, item: KanaClass) -> String {
    match method.kana_table().get(&item) {
        Some(latin) => (*latin).to_string(),
        None => item.lisp_name().to_ascii_lowercase(),
    }
}

/// `r-apply` (gf — `romanize.lisp:44-55, 69-77, 106-130`). Apply a
/// modifier to its wrapped subtree. `:sokuon` doubles the lead
/// consonant (or prefixes `t` before `chi` under hepburn);
/// `:long-vowel` returns the inner romanization; small-form vowel /
/// y-glide modifiers consult the kana-table — hepburn's
/// `sha`/`cha`/`ja` family overrides the y-glides over
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

/// generic-hepburn `:+ya`/`:+yu`/`:+yo` overrides (`romanize.lisp:111-130`).
/// `None` falls through to the generic-romanization method.
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

/// `r-simplify` (gf — `romanize.lisp:57-59, 132-199`). Post-process per
/// the method's orthography. The default `(method str)` is identity;
/// each method class overrides it.
pub fn r_simplify(method: RomanizationMethod<'_>, str: &str) -> String {
    match method {
        RomanizationMethod::GenericHepburn(method) => method.r_simplify(str),
        RomanizationMethod::SimplifiedHepburn(method) => method.r_simplify(str),
        RomanizationMethod::TraditionalHepburn(method) => method.r_simplify(str),
        RomanizationMethod::ModifiedHepburn(method) => method.r_simplify(str),
        RomanizationMethod::KunreiSiki(method) => method.r_simplify(str),
    }
}

/// `r-special` (gf — `romanize.lisp:210-215`). Romanize standalone
/// glyphs that carry no mora class. Only the default `or` method
/// exists, so the result is method-independent.
pub fn r_special(method: RomanizationMethod<'_>, word: &str) -> Option<String> {
    let _ = method;
    match word {
        "っ" => Some("!".to_string()),
        "ー" => Some("~".to_string()),
        _ => None,
    }
}

/// `romanize-core` (`romanize.lisp:31-37`). Walk a character-class
/// tree, concatenating each node's romanization. `nil` drops out, a
/// raw character passes through, a mora class goes through `r_base`, a
/// modifier node through `r_apply`.
pub fn romanize_core(method: RomanizationMethod<'_>, cc_tree: &[CcTree]) -> String {
    let mut out = String::new();
    for item in cc_tree {
        match item {
            CcTree::Nil => {}
            CcTree::Atom(CcItem::Char(character)) => out.push(*character),
            CcTree::Atom(CcItem::Class(class)) => out.push_str(&r_base(method, *class)),
            CcTree::Node(modifier, tail) => out.push_str(&r_apply(*modifier, method, tail)),
        }
    }
    out
}

/// `join-parts` (`romanize.lisp:235-246`). Concatenate `parts`,
/// inserting a single space before a part that begins with an
/// alphanumeric character when the running output didn't already end
/// in whitespace. Empty parts neither trigger a space nor update the
/// flag. Flag starts true, so the first part never gets a leading
/// space.
pub fn join_parts<S: AsRef<str>>(parts: &[S]) -> String {
    let mut out = String::new();
    let mut last_space = true;
    for part in parts {
        let part = part.as_ref();
        let chars: Vec<char> = part.chars().collect();
        let len = chars.len();
        if len != 0 && !last_space && alphanumericp(chars[0]) {
            out.push(' ');
        }
        out.push_str(part);
        if len != 0 {
            last_space = chars[len - 1].is_whitespace();
        }
    }
    out
}

/// Lisp `alphanumericp` — `alpha-char-p` (Lu/Ll/Lt/Lm/Lo) or
/// `digit-char-p` radix 10 (Nd). Std `is_alphanumeric` includes Nl/No
/// too, which Lisp rejects.
fn alphanumericp(char: char) -> bool {
    char.general_category_group() == GeneralCategoryGroup::Letter
        || char.general_category() == GeneralCategory::DecimalNumber
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::methods::{
        GenericHepburn, GenericRomanization, KunreiSiki, SimplifiedHepburn, TraditionalHepburn,
    };

    fn cls(kana: KanaClass) -> CcItem {
        CcItem::Class(kana)
    }
    fn atom(kana: KanaClass) -> CcTree {
        CcTree::Atom(CcItem::Class(kana))
    }
    fn chr(character: char) -> CcTree {
        CcTree::Atom(CcItem::Char(character))
    }
    fn node(kana: KanaClass, tail: Vec<CcTree>) -> CcTree {
        CcTree::Node(kana, tail)
    }

    /// REPL fixtures (.103, ichiran::get-character-classes), 2026-05-23.
    #[test]
    fn get_character_classes_fixtures() {
        use KanaClass::*;
        let cases: Vec<(&str, Vec<CcItem>)> = vec![
            ("し", vec![cls(Shi)]),
            ("による", vec![cls(Ni), cls(Yo), cls(Ru)]),
            ("コーヒー", vec![cls(Ko), cls(LongVowel), cls(Hi), cls(LongVowel)]),
            ("きっぷ", vec![cls(Ki), cls(Sokuon), cls(Pu)]),
            ("ゝゞ", vec![cls(Iter), cls(IterV)]),
            ("Aと5", vec![CcItem::Char('A'), cls(To), CcItem::Char('5')]),
            ("東京", vec![CcItem::Char('東'), CcItem::Char('京')]),
        ];
        for (word, expected) in &cases {
            assert_eq!(&get_character_classes(word), expected, "word={word:?}");
        }
    }

    #[test]
    fn iter_at_start_emits_nothing() {
        let result = process_iteration_characters(&[cls(KanaClass::Iter), cls(KanaClass::IterV)]);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn iter_repeats_previous_item() {
        let result = process_iteration_characters(&[cls(KanaClass::Sa), cls(KanaClass::Iter)]);
        assert_eq!(result, vec![cls(KanaClass::Sa), cls(KanaClass::Sa)]);
    }

    #[test]
    fn iter_v_voices_previous_kana() {
        let result = process_iteration_characters(&[cls(KanaClass::Sa), cls(KanaClass::IterV)]);
        assert_eq!(result, vec![cls(KanaClass::Sa), cls(KanaClass::Za)]);
    }

    #[test]
    fn run_of_iters_all_reference_same_source() {
        let result = process_iteration_characters(&[
            cls(KanaClass::Sa),
            cls(KanaClass::Iter),
            cls(KanaClass::Iter),
            cls(KanaClass::Iter),
        ]);
        assert_eq!(
            result,
            vec![
                cls(KanaClass::Sa),
                cls(KanaClass::Sa),
                cls(KanaClass::Sa),
                cls(KanaClass::Sa),
            ]
        );
    }

    #[test]
    fn iter_v_after_unvoiceable_kana_falls_through() {
        let result = process_iteration_characters(&[cls(KanaClass::A), cls(KanaClass::IterV)]);
        assert_eq!(result, vec![cls(KanaClass::A), cls(KanaClass::A)]);
    }

    #[test]
    fn char_prev_passes_through_iter_v_unchanged() {
        let result = process_iteration_characters(&[CcItem::Char('!'), cls(KanaClass::IterV)]);
        assert_eq!(result, vec![CcItem::Char('!'), CcItem::Char('!')]);
    }

    /// REPL fixtures (.103, ichiran::process-modifiers over
    /// process-iteration-characters of the cited word), 2026-05-23.
    #[test]
    fn process_modifiers_fixtures() {
        use KanaClass::*;
        let cases: Vec<(&str, Vec<CcItem>, Vec<CcTree>)> = vec![
            ("きっぷ", vec![cls(Ki), cls(Sokuon), cls(Pu)],
                vec![atom(Ki), node(Sokuon, vec![atom(Pu)])]),
            ("きゃく", vec![cls(Ki), cls(PlusYa), cls(Ku)],
                vec![node(PlusYa, vec![atom(Ki)]), atom(Ku)]),
            ("コーヒー", vec![cls(Ko), cls(LongVowel), cls(Hi), cls(LongVowel)],
                vec![node(LongVowel, vec![atom(Ko)]), node(LongVowel, vec![atom(Hi)])]),
            ("ぁ", vec![cls(PlusA)],
                vec![node(PlusA, vec![CcTree::Nil])]),
            ("ゃゅ", vec![cls(PlusYa), cls(PlusYu)],
                vec![node(PlusYu, vec![node(PlusYa, vec![CcTree::Nil])])]),
            ("っ", vec![cls(Sokuon)],
                vec![node(Sokuon, vec![])]),
            ("がっこう", vec![cls(Ga), cls(Sokuon), cls(Ko), cls(U)],
                vec![atom(Ga), node(Sokuon, vec![atom(Ko), atom(U)])]),
            ("しゃっくり", vec![cls(Shi), cls(PlusYa), cls(Sokuon), cls(Ku), cls(Ri)],
                vec![node(PlusYa, vec![atom(Shi)]), node(Sokuon, vec![atom(Ku), atom(Ri)])]),
            ("チョコレート", vec![cls(Chi), cls(PlusYo), cls(Ko), cls(Re), cls(LongVowel), cls(To)],
                vec![node(PlusYo, vec![atom(Chi)]), atom(Ko), node(LongVowel, vec![atom(Re)]), atom(To)]),
            ("Aと5", vec![CcItem::Char('A'), cls(To), CcItem::Char('5')],
                vec![chr('A'), atom(To), chr('5')]),
        ];
        for (label, input, expected) in &cases {
            assert_eq!(&process_modifiers(input), expected, "case={label:?}");
        }
    }

    /// REPL fixtures (.103, ichiran::leftmost-atom), 2026-05-23.
    #[test]
    fn leftmost_atom_fixtures() {
        use KanaClass::*;
        let cases: Vec<(&str, Vec<CcTree>, Option<CcItem>)> = vec![
            ("(:TA)", vec![atom(Ta)], Some(cls(Ta))),
            ("(:SO :U :SHI)", vec![atom(So), atom(U), atom(Shi)], Some(cls(So))),
            ("((:+YA :CHI))", vec![node(PlusYa, vec![atom(Chi)])], Some(cls(Chi))),
            ("((:SOKUON (:+YA :CHI)))",
                vec![node(Sokuon, vec![node(PlusYa, vec![atom(Chi)])])],
                Some(cls(Chi))),
            ("((:+YU (:+YA :CHI)))",
                vec![node(PlusYu, vec![node(PlusYa, vec![atom(Chi)])])],
                Some(cls(Chi))),
            ("NIL", vec![], None),
            ("((:+YA NIL))", vec![node(PlusYa, vec![CcTree::Nil])], None),
            ("(#\\a)", vec![CcTree::Atom(CcItem::Char('a'))], Some(CcItem::Char('a'))),
        ];
        for (label, input, expected) in &cases {
            assert_eq!(&leftmost_atom(input), expected, "case={label}");
        }
    }

    /// REPL fixtures (.103, ichiran::r-base), 2026-05-24.
    #[test]
    fn r_base_fixtures() {
        use KanaClass::*;
        let hepburn = GenericHepburn::new();
        let kunrei = KunreiSiki::new();
        let method_hepburn = RomanizationMethod::GenericHepburn(&hepburn);
        let method_kunrei = RomanizationMethod::KunreiSiki(&kunrei);
        assert_eq!(r_base(method_hepburn, Ka), "ka");
        assert_eq!(r_base(method_hepburn, N), "n'");
        assert_eq!(r_base(method_kunrei, Shi), "si");
    }

    /// REPL fixtures (.103, (r-base (make-instance 'generic-romanization) X)), 2026-05-24.
    #[test]
    fn r_base_downcase_fallback() {
        use KanaClass::*;
        let mut empty = GenericHepburn::new();
        empty.0 = GenericRomanization::new();
        let method = RomanizationMethod::GenericHepburn(&empty);
        assert_eq!(r_base(method, N), "n");
        assert_eq!(r_base(method, Ka), "ka");
        assert_eq!(r_base(method, PlusYa), "+ya");
    }

    /// REPL fixtures (.103, ichiran::r-apply), 2026-05-24.
    #[test]
    fn r_apply_fixtures() {
        use KanaClass::*;
        let hepburn = GenericHepburn::new();
        let kunrei = KunreiSiki::new();
        let mut bare = GenericHepburn::new();
        bare.0 = GenericRomanization::new();
        let h = RomanizationMethod::GenericHepburn(&hepburn);
        let k = RomanizationMethod::KunreiSiki(&kunrei);
        let b = RomanizationMethod::GenericHepburn(&bare);
        let cases: &[(&str, KanaClass, RomanizationMethod, Vec<CcTree>, &str)] = &[
            ("sokuon hepburn chi", Sokuon, h, vec![atom(Chi)], "tchi"),
            ("sokuon hepburn pu", Sokuon, h, vec![atom(Pu)], "ppu"),
            ("sokuon hepburn empty", Sokuon, h, vec![], ""),
            ("sokuon hepburn cyrillic", Sokuon, h, vec![chr('я')], "я"),
            ("sokuon kunrei chi", Sokuon, k, vec![atom(Chi)], "tti"),
            ("sokuon kunrei pu", Sokuon, k, vec![atom(Pu)], "ppu"),
            ("long-vowel hepburn ko", LongVowel, h, vec![atom(Ko)], "ko"),
            ("+ya hepburn shi", PlusYa, h, vec![atom(Shi)], "sha"),
            ("+ya hepburn chi", PlusYa, h, vec![atom(Chi)], "cha"),
            ("+ya hepburn ji", PlusYa, h, vec![atom(Ji)], "ja"),
            ("+ya hepburn dji", PlusYa, h, vec![atom(Dji)], "ja"),
            ("+ya hepburn ki", PlusYa, h, vec![atom(Ki)], "kya"),
            ("+yu hepburn shi", PlusYu, h, vec![atom(Shi)], "shu"),
            ("+yu hepburn ki", PlusYu, h, vec![atom(Ki)], "kyu"),
            ("+yo hepburn chi", PlusYo, h, vec![atom(Chi)], "cho"),
            ("+yo hepburn ki", PlusYo, h, vec![atom(Ki)], "kyo"),
            ("+ya kunrei shi", PlusYa, k, vec![atom(Shi)], "sya"),
            ("+ya kunrei ki", PlusYa, k, vec![atom(Ki)], "kya"),
            ("+a hepburn u", PlusA, h, vec![atom(U)], "wa"),
            ("+a hepburn a", PlusA, h, vec![atom(A)], "aa"),
            ("+i hepburn i", PlusI, h, vec![atom(I)], "ii"),
            ("+wa hepburn ku", PlusWa, h, vec![atom(Ku)], "kwa"),
            ("+a hepburn ki", PlusA, h, vec![atom(Ki)], "ka"),
            ("+ya bare ki", PlusYa, b, vec![atom(Ki)], "ki+ya"),
        ];
        for (label, modifier, method, cc_tree, expected) in cases {
            assert_eq!(&r_apply(*modifier, *method, cc_tree), expected, "case={label}");
        }
    }

    /// REPL fixtures (.103, ichiran::r-simplify), 2026-05-24.
    #[test]
    fn r_simplify_fixtures() {
        let generic = GenericHepburn::new();
        let simple = SimplifiedHepburn::new(vec!["oo", "o", "ou", "o", "uu", "u"]);
        let passport = SimplifiedHepburn::new(vec!["oo", "oh", "ou", "oh", "uu", "u"]);
        let traditional = TraditionalHepburn::new();
        let kunrei = KunreiSiki::new();
        let hepburn = RomanizationMethod::GenericHepburn(&generic);
        let simple = RomanizationMethod::SimplifiedHepburn(&simple);
        let passport = RomanizationMethod::SimplifiedHepburn(&passport);
        let traditional = RomanizationMethod::TraditionalHepburn(&traditional);
        let kunrei = RomanizationMethod::KunreiSiki(&kunrei);
        let cases: &[(&str, RomanizationMethod, &str, &str)] = &[
            ("hepburn n'+consonant", hepburn, "kon'nichiwa", "konnichiwa"),
            ("hepburn n'+vowel", hepburn, "han'i", "han'i"),
            ("hepburn n'+y", hepburn, "shin'you", "shin'you"),
            ("simple long-o", simple, "koukou", "koko"),
            ("simple n'+ngram", simple, "n'pou", "npo"),
            ("passport long-o", passport, "koukou", "kohkoh"),
            ("traditional long-o", traditional, "koukou", "kōkō"),
            ("traditional n+b", traditional, "shinbun", "shimbun"),
            ("traditional n'+vowel", traditional, "kon'i", "kon-i"),
            ("traditional n+m", traditional, "honma", "homma"),
            ("traditional n'+y", traditional, "shin'you", "shin-yō"),
            ("traditional n+p", traditional, "kanpeki", "kampeki"),
            ("kunrei long-o", kunrei, "koukou", "kôkô"),
            ("kunrei n'+consonant", kunrei, "kon'nichi", "konnichi"),
        ];
        for (label, method, input, expected) in cases {
            assert_eq!(&r_simplify(*method, input), expected, "case={label}");
        }
    }

    /// REPL fixtures (.103, ichiran::r-special), 2026-05-24.
    #[test]
    fn r_special_fixtures() {
        let traditional = TraditionalHepburn::new();
        let method = RomanizationMethod::TraditionalHepburn(&traditional);
        assert_eq!(r_special(method, "っ"), Some("!".to_string()));
        assert_eq!(r_special(method, "ー"), Some("~".to_string()));
        assert_eq!(r_special(method, "あ"), None);
    }

    /// REPL (.103), 2026-05-24. Exercises mora class, nil skip, raw char
    /// passthrough, and a modifier node in one tree.
    #[test]
    fn romanize_core_walks_every_node_shape() {
        use KanaClass::*;
        let hepburn = GenericHepburn::new();
        let method = RomanizationMethod::GenericHepburn(&hepburn);
        let cc_tree = vec![
            CcTree::Atom(CcItem::Class(Ko)),
            CcTree::Nil,
            CcTree::Atom(CcItem::Class(N)),
            CcTree::Atom(CcItem::Char('x')),
            CcTree::Node(PlusYa, vec![CcTree::Atom(CcItem::Class(Ki))]),
        ];
        assert_eq!(romanize_core(method, &cc_tree), "kon'xkya");
    }

    /// REPL fixtures (.103, ichiran::join-parts), 2026-05-23.
    #[test]
    fn join_parts_fixtures() {
        let cases: &[(&[&str], &str)] = &[
            (&["watashi", "wa", "gakusei", "desu"], "watashi wa gakusei desu"),
            (&["Tokyo", ",", "desu"], "Tokyo, desu"),
            (&["hello ", "world"], "hello world"),
            (&["", "abc"], "abc"),
            (&["abc", "", "def"], "abc def"),
            (&["Tokyo", "。"], "Tokyo。"),
            (&["a", "①"], "a①"),
            (&["Ⅴ", "a"], "Ⅴ a"),
            (&["a", "5"], "a 5"),
            (&["a", "５"], "a ５"),
            (&["a", "ー"], "a ー"),
            (&["foo　", "bar"], "foo　bar"),
        ];
        for (parts, expected) in cases {
            assert_eq!(&join_parts(parts), expected, "parts={parts:?}");
        }
    }
}
