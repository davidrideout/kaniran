use super::*;
use crate::characters::kani_kana_class::KanaClass;

// --- generic_romanization_class ---
#[test]
fn base_kana_table_is_empty() {
    assert_eq!(GenericRomanization::new().kana_table.len(), 0);
}

// --- generic_hepburn_class ---
#[test]
fn carries_a_copy_of_the_hepburn_table() {
    let method = GenericHepburn::new();
    assert_eq!(method.0.kana_table.len(), 83);
    assert_eq!(method.0.kana_table.get(&KanaClass::Shi), Some(&"shi"));
}

// --- simplified_hepburn_class ---
#[test]
fn default_simplifications_is_empty() {
    let method = SimplifiedHepburn::new(Vec::new());
    assert!(method.simplifications.is_empty());
    assert_eq!(method.base.0.kana_table.len(), 83);
}

#[test]
fn initarg_simplifications_pass_through() {
    let method = SimplifiedHepburn::new(vec!["xx", "y"]);
    assert_eq!(method.simplifications, vec!["xx", "y"]);
}

// --- traditional_hepburn_class ---
#[test]
fn traditional_hepburn_class_redefined_simplifications_initform() {
    assert_eq!(
        TraditionalHepburn::new().0.simplifications,
        vec!["oo", "ō", "ou", "ō", "uu", "ū"]
    );
}

#[test]
fn inherits_the_hepburn_kana_table() {
    // Inherits generic-hepburn's kana table unchanged — not kunrei, and
    // without the modified-hepburn :wo override.
    let method = TraditionalHepburn::new();
    let kana_table = &method.0.base.0.kana_table;
    assert_eq!(kana_table.len(), 83);
    assert_eq!(kana_table.get(&KanaClass::Shi), Some(&"shi"));
    assert_eq!(kana_table.get(&KanaClass::Wo), Some(&"wo"));
    assert_eq!(kana_table.get(&KanaClass::Ji), Some(&"ji"));
}

// --- modified_hepburn_class ---
#[test]
fn modified_hepburn_class_redefined_simplifications_initform() {
    assert_eq!(
        ModifiedHepburn::new().0.simplifications,
        vec!["oo", "ō", "ou", "ō", "uu", "ū", "aa", "ā", "ee", "ē"]
    );
}

#[test]
fn wo_override_on_inherited_hepburn_table() {
    // Overrides wo to "o"; the rest of the table is the inherited hepburn copy.
    let method = ModifiedHepburn::new();
    let kana_table = &method.0.base.0.kana_table;
    assert_eq!(kana_table.len(), 83);
    assert_eq!(kana_table.get(&KanaClass::Wo), Some(&"o"));
    assert_eq!(kana_table.get(&KanaClass::Shi), Some(&"shi"));
    assert_eq!(kana_table.get(&KanaClass::Ji), Some(&"ji"));
    assert_eq!(kana_table.get(&KanaClass::Wi), Some(&"wi"));
}

#[test]
fn r_simplify_inherits_simplified_hepburn() {
    // Modified-hepburn simplification: drops n' and folds long vowels,
    // but does not apply traditional's n->m rule.
    let method = ModifiedHepburn::new();
    let cases: &[(&str, &str)] = &[
        ("koukou", "kōkō"),
        ("okaasan", "okāsan"),
        ("oneesan", "onēsan"),
        ("suugaku", "sūgaku"),
        ("kon'nichiwa", "konnichiwa"),
        ("han'i", "han'i"),
        // no n->m fold (that is traditional-only)
        ("shinbun", "shinbun"),
        ("honma", "honma"),
    ];
    for (input, expected) in cases {
        assert_eq!(&method.r_simplify(input), expected, "input={input}");
    }
}

// --- kunrei_siki_class ---
#[test]
fn carries_a_copy_of_the_kunrei_table() {
    let method = KunreiSiki::new();
    let kana_table = &method.0.kana_table;
    assert_eq!(kana_table.len(), 83);
    assert_eq!(kana_table.get(&KanaClass::Shi), Some(&"si"));
    assert_eq!(kana_table.get(&KanaClass::Ji), Some(&"zi"));
    assert_eq!(kana_table.get(&KanaClass::Fu), Some(&"hu"));
    assert_eq!(kana_table.get(&KanaClass::Wo), Some(&"o"));
    assert_eq!(kana_table.get(&KanaClass::Wi), Some(&"i"));
    assert_eq!(kana_table.get(&KanaClass::We), Some(&"e"));
}

// --- _star_default_romanization_method_star_ ---
#[test]
fn is_the_same_instance_as_hepburn_traditional() {
    assert!(std::ptr::eq(
        default_romanization_method(),
        hepburn_traditional()
    ));
}
