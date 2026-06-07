use super::*;
use crate::core::methods::{
    GenericHepburn, GenericRomanization, KunreiSiki, SimplifiedHepburn, TraditionalHepburn,
};

// --- r_apply ---
fn r_apply_atom(class: KanaClass) -> CcTree {
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
    // An emptied table reaches the downcase fallback; a populated table
    // never hits that branch.
    let mut bare = GenericHepburn::new();
    bare.0 = GenericRomanization::new();
    let h = RomanizationMethod::GenericHepburn(&hepburn);
    let k = RomanizationMethod::KunreiSiki(&kunrei);
    let b = RomanizationMethod::GenericHepburn(&bare);
    // Columns: label, modifier, method, cc-tree, expected.
    let cases: &[(&str, KanaClass, RomanizationMethod, Vec<CcTree>, &str)] = &[
        // sokuon: hepburn chi -> "t", else double a Basic-Latin lead
        (
            "sokuon hepburn chi",
            Sokuon,
            h,
            vec![r_apply_atom(Chi)],
            "tchi",
        ),
        (
            "sokuon hepburn pu",
            Sokuon,
            h,
            vec![r_apply_atom(Pu)],
            "ppu",
        ),
        ("sokuon hepburn empty", Sokuon, h, vec![], ""),
        ("sokuon hepburn cyrillic", Sokuon, h, vec![chr('я')], "я"),
        (
            "sokuon kunrei chi",
            Sokuon,
            k,
            vec![r_apply_atom(Chi)],
            "tti",
        ),
        ("sokuon kunrei pu", Sokuon, k, vec![r_apply_atom(Pu)], "ppu"),
        // long-vowel: inner romanization unchanged
        (
            "long-vowel hepburn ko",
            LongVowel,
            h,
            vec![r_apply_atom(Ko)],
            "ko",
        ),
        // hepburn y-glide overrides over shi/chi/ji/dji
        ("+ya hepburn shi", PlusYa, h, vec![r_apply_atom(Shi)], "sha"),
        ("+ya hepburn chi", PlusYa, h, vec![r_apply_atom(Chi)], "cha"),
        ("+ya hepburn ji", PlusYa, h, vec![r_apply_atom(Ji)], "ja"),
        ("+ya hepburn dji", PlusYa, h, vec![r_apply_atom(Dji)], "ja"),
        ("+ya hepburn ki", PlusYa, h, vec![r_apply_atom(Ki)], "kya"),
        ("+yu hepburn shi", PlusYu, h, vec![r_apply_atom(Shi)], "shu"),
        ("+yu hepburn ki", PlusYu, h, vec![r_apply_atom(Ki)], "kyu"),
        ("+yo hepburn chi", PlusYo, h, vec![r_apply_atom(Chi)], "cho"),
        ("+yo hepburn ki", PlusYo, h, vec![r_apply_atom(Ki)], "kyo"),
        // kunrei has no y-glide override
        ("+ya kunrei shi", PlusYa, k, vec![r_apply_atom(Shi)], "sya"),
        ("+ya kunrei ki", PlusYa, k, vec![r_apply_atom(Ki)], "kya"),
        // generic modifier cases: u, the a/i/e/o vowels, and the default
        ("+a hepburn u", PlusA, h, vec![r_apply_atom(U)], "wa"),
        ("+a hepburn a", PlusA, h, vec![r_apply_atom(A)], "aa"),
        ("+i hepburn i", PlusI, h, vec![r_apply_atom(I)], "ii"),
        ("+wa hepburn ku", PlusWa, h, vec![r_apply_atom(Ku)], "kwa"),
        ("+a hepburn ki", PlusA, h, vec![r_apply_atom(Ki)], "ka"),
        // fallback: modifier missing from the table -> downcase
        ("+ya bare ki", PlusYa, b, vec![r_apply_atom(Ki)], "ki+ya"),
    ];
    for (label, modifier, method, cc_tree, expected) in cases {
        assert_eq!(
            &r_apply(*modifier, *method, cc_tree),
            expected,
            "case={label}"
        );
    }
}

// --- r_base ---
#[test]
fn r_base_fixtures() {
    use KanaClass::*;
    let hepburn = GenericHepburn::new();
    let kunrei = KunreiSiki::new();
    let method_hepburn = RomanizationMethod::GenericHepburn(&hepburn);
    let method_kunrei = RomanizationMethod::KunreiSiki(&kunrei);
    // table hits
    assert_eq!(r_base(method_hepburn, Ka), "ka");
    assert_eq!(r_base(method_hepburn, N), "n'");
    assert_eq!(r_base(method_kunrei, Shi), "si");
}

#[test]
fn r_base_downcase_fallback() {
    use KanaClass::*;
    // With an empty kana table, r_base misses and falls back to the
    // downcased key name.
    let mut empty = GenericHepburn::new();
    empty.0 = GenericRomanization::new();
    let method = RomanizationMethod::GenericHepburn(&empty);
    assert_eq!(r_base(method, N), "n");
    assert_eq!(r_base(method, Ka), "ka");
    assert_eq!(r_base(method, PlusYa), "+ya");
}

// --- r_simplify ---
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
    // Columns: label, method, input, expected.
    let cases: &[(&str, RomanizationMethod, &str, &str)] = &[
        // generic-hepburn: n' drops before a non-vowel, stays before vowel/y
        ("hepburn n'+consonant", hepburn, "kon'nichiwa", "konnichiwa"),
        ("hepburn n'+vowel", hepburn, "han'i", "han'i"),
        ("hepburn n'+y", hepburn, "shin'you", "shin'you"),
        // simplified-hepburn: applies the configured simplifications
        ("simple long-o", simple, "koukou", "koko"),
        ("simple n'+ngram", simple, "n'pou", "npo"),
        ("passport long-o", passport, "koukou", "kohkoh"),
        // traditional-hepburn: macron fold + n-/m rules
        ("traditional long-o", traditional, "koukou", "kōkō"),
        ("traditional n+b", traditional, "shinbun", "shimbun"),
        ("traditional n'+vowel", traditional, "kon'i", "kon-i"),
        ("traditional n+m", traditional, "honma", "homma"),
        ("traditional n'+y", traditional, "shin'you", "shin-yō"),
        ("traditional n+p", traditional, "kanpeki", "kampeki"),
        // kunrei-siki: inline n' drop + circumflex long-vowel fold
        ("kunrei long-o", kunrei, "koukou", "kôkô"),
        ("kunrei n'+consonant", kunrei, "kon'nichi", "konnichi"),
    ];
    for (label, method, input, expected) in cases {
        assert_eq!(&r_simplify(*method, input), expected, "case={label}");
    }
}

// --- r_special ---
#[test]
fn r_special_fixtures() {
    // Result is the same regardless of method — only one method exists.
    let traditional = TraditionalHepburn::new();
    let method = RomanizationMethod::TraditionalHepburn(&traditional);
    assert_eq!(r_special(method, "っ"), Some("!".to_string()));
    assert_eq!(r_special(method, "ー"), Some("~".to_string()));
    assert_eq!(r_special(method, "あ"), None);
}

// --- process_iteration_characters ---
fn k(c: KanaClass) -> CcItem {
    CcItem::Class(c)
}

#[test]
fn iter_at_start_emits_nothing() {
    // No prev yet — both markers drop out, no panic.
    let result = process_iteration_characters(&[k(KanaClass::Iter), k(KanaClass::IterV)]);
    assert_eq!(result, vec![]);
}

#[test]
fn iter_repeats_previous_item() {
    // さゝ — Sa, then Iter expands to a second Sa.
    let result = process_iteration_characters(&[k(KanaClass::Sa), k(KanaClass::Iter)]);
    assert_eq!(result, vec![k(KanaClass::Sa), k(KanaClass::Sa)]);
}

#[test]
fn iter_v_voices_previous_kana() {
    // さゞ — Sa, then IterV expands to the voiced form Za.
    let result = process_iteration_characters(&[k(KanaClass::Sa), k(KanaClass::IterV)]);
    assert_eq!(result, vec![k(KanaClass::Sa), k(KanaClass::Za)]);
}

#[test]
fn run_of_iters_all_reference_same_source() {
    // prev only updates on a non-iteration item, so さゝゝゝ all
    // expand from the original Sa, not from each emitted copy.
    let result = process_iteration_characters(&[
        k(KanaClass::Sa),
        k(KanaClass::Iter),
        k(KanaClass::Iter),
        k(KanaClass::Iter),
    ]);
    assert_eq!(
        result,
        vec![
            k(KanaClass::Sa),
            k(KanaClass::Sa),
            k(KanaClass::Sa),
            k(KanaClass::Sa),
        ]
    );
}

#[test]
fn iter_v_after_unvoiceable_kana_falls_through() {
    // A has no voiced counterpart, so the voicing marker leaves it unchanged.
    let result = process_iteration_characters(&[k(KanaClass::A), k(KanaClass::IterV)]);
    assert_eq!(result, vec![k(KanaClass::A), k(KanaClass::A)]);
}

#[test]
fn char_prev_passes_through_iter_v_unchanged() {
    // A raw char has no voiced form, so the voicing marker emits the char unchanged.
    let result = process_iteration_characters(&[CcItem::Char('!'), k(KanaClass::IterV)]);
    assert_eq!(result, vec![CcItem::Char('!'), CcItem::Char('!')]);
}

// --- process_modifiers ---
fn cls(kana: KanaClass) -> CcItem {
    CcItem::Class(kana)
}
fn process_modifiers_atom(kana: KanaClass) -> CcTree {
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
    // Each row is (label, input cc-list, expected cc-tree).
    let cases: Vec<(&str, Vec<CcItem>, Vec<CcTree>)> = vec![
        // sokuon wraps the rest
        (
            "きっぷ",
            vec![cls(Ki), cls(Sokuon), cls(Pu)],
            vec![
                process_modifiers_atom(Ki),
                node(Sokuon, vec![process_modifiers_atom(Pu)]),
            ],
        ),
        // modifier wraps the preceding item
        (
            "きゃく",
            vec![cls(Ki), cls(PlusYa), cls(Ku)],
            vec![
                node(PlusYa, vec![process_modifiers_atom(Ki)]),
                process_modifiers_atom(Ku),
            ],
        ),
        // long-vowel modifiers
        (
            "コーヒー",
            vec![cls(Ko), cls(LongVowel), cls(Hi), cls(LongVowel)],
            vec![
                node(LongVowel, vec![process_modifiers_atom(Ko)]),
                node(LongVowel, vec![process_modifiers_atom(Hi)]),
            ],
        ),
        // leading modifier: nothing to pop, slot is nil
        ("ぁ", vec![cls(PlusA)], vec![node(PlusA, vec![CcTree::Nil])]),
        // nested modifiers: pop a node
        (
            "ゃゅ",
            vec![cls(PlusYa), cls(PlusYu)],
            vec![node(PlusYu, vec![node(PlusYa, vec![CcTree::Nil])])],
        ),
        // sokuon at end wraps the empty rest
        ("っ", vec![cls(Sokuon)], vec![node(Sokuon, vec![])]),
        // sokuon with following mora
        (
            "がっこう",
            vec![cls(Ga), cls(Sokuon), cls(Ko), cls(U)],
            vec![
                process_modifiers_atom(Ga),
                node(
                    Sokuon,
                    vec![process_modifiers_atom(Ko), process_modifiers_atom(U)],
                ),
            ],
        ),
        // modifier then sokuon
        (
            "しゃっくり",
            vec![cls(Shi), cls(PlusYa), cls(Sokuon), cls(Ku), cls(Ri)],
            vec![
                node(PlusYa, vec![process_modifiers_atom(Shi)]),
                node(
                    Sokuon,
                    vec![process_modifiers_atom(Ku), process_modifiers_atom(Ri)],
                ),
            ],
        ),
        // modifier + long-vowel mix
        (
            "チョコレート",
            vec![
                cls(Chi),
                cls(PlusYo),
                cls(Ko),
                cls(Re),
                cls(LongVowel),
                cls(To),
            ],
            vec![
                node(PlusYo, vec![process_modifiers_atom(Chi)]),
                process_modifiers_atom(Ko),
                node(LongVowel, vec![process_modifiers_atom(Re)]),
                process_modifiers_atom(To),
            ],
        ),
        // non-kana chars pass through as plain atoms
        (
            "Aと5",
            vec![CcItem::Char('A'), cls(To), CcItem::Char('5')],
            vec![ch('A'), process_modifiers_atom(To), ch('5')],
        ),
    ];
    for (label, input, expected) in &cases {
        assert_eq!(&process_modifiers(input), expected, "case={label:?}");
    }
}

// --- apply_rmap_item ---
fn rmi(text: &str, kana: &str, next: Option<&str>) -> RmapItem {
    RmapItem {
        text: text.to_string(),
        kana: kana.to_string(),
        next: next.map(str::to_string),
    }
}

#[test]
fn apply_rmap_item_fixtures() {
    // Columns: input string, rmap item, expected (canonical, pattern, rest).
    let cases: &[(&str, RmapItem, (&str, &str, &str))] = &[
        // long-vowel branch: text ends in o/u -> pattern gets う?
        (
            "konnichiwa",
            rmi("ko", "こ", None),
            ("こ", "こう?", "nnichiwa"),
        ),
        ("ohayou", rmi("o", "お", None), ("お", "おう?", "hayou")),
        ("toukyou", rmi("to", "と", None), ("と", "とう?", "ukyou")),
        // plain branch: text ends in other char
        ("katana", rmi("ka", "か", None), ("か", "か", "tana")),
        (
            "shinkansen",
            rmi("shi", "し", None),
            ("し", "し", "nkansen"),
        ),
        ("nagoya", rmi("na", "な", None), ("な", "な", "goya")),
        // gemination: next re-emitted before the unconsumed tail
        ("kkon", rmi("kk", "っ", Some("k")), ("っ", "っ", "kon")),
        ("ppai", rmi("pp", "っ", Some("p")), ("っ", "っ", "pai")),
        ("kkou", rmi("kk", "っ", Some("k")), ("っ", "っ", "kou")),
    ];
    for (s, item, (canonical, pattern, rest)) in cases {
        let got = apply_rmap_item(s, item);
        assert_eq!(got.canonical, *canonical, "canonical, s={s:?} rmi={item:?}");
        assert_eq!(got.pattern, *pattern, "pattern, s={s:?} rmi={item:?}");
        assert_eq!(got.rest, *rest, "rest, s={s:?} rmi={item:?}");
        assert_eq!(got.branch, 0, "branch, s={s:?} rmi={item:?}");
    }
}
