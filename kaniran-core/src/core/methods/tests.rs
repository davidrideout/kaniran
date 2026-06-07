use super::*;
use crate::characters::kani_kana_class::KanaClass;

// --- generic_romanization_class ---
#[test]
fn base_kana_table_is_empty() {
    // romanize.lisp:63-64 — (kana-table :initform (make-hash-table)).
    // REPL (.103, make-instance 'generic-romanization): kana-count=0.
    assert_eq!(GenericRomanization::new().kana_table.len(), 0);
}

// --- generic_hepburn_class ---
#[test]
fn carries_a_copy_of_the_hepburn_table() {
    // romanize.lisp:104 — kana-table is a copy-hash-table of *hepburn-kana-table*.
    // REPL (.103, make-instance 'generic-hepburn): kana-count=83, :shi="shi".
    let method = GenericHepburn::new();
    assert_eq!(method.0.kana_table.len(), 83);
    assert_eq!(method.0.kana_table.get(&KanaClass::Shi), Some(&"shi"));
}

// --- simplified_hepburn_class ---
#[test]
fn default_simplifications_is_empty() {
    // romanize.lisp:137 — (simplifications :initform nil).
    // REPL (.103, make-instance 'simplified-hepburn): simpl=NIL.
    let method = SimplifiedHepburn::new(Vec::new());
    assert!(method.simplifications.is_empty());
    assert_eq!(method.base.0.kana_table.len(), 83);
}

#[test]
fn initarg_simplifications_pass_through() {
    // romanize.lisp:137 — (simplifications :initarg :simplifications).
    // REPL (.103, make-instance 'simplified-hepburn :simplifications '("xx" "y")):
    // simpl=("xx" "y").
    let method = SimplifiedHepburn::new(vec!["xx", "y"]);
    assert_eq!(method.simplifications, vec!["xx", "y"]);
}

// --- traditional_hepburn_class ---
#[test]
fn traditional_hepburn_class_redefined_simplifications_initform() {
    // romanize.lisp:153 — simplifications initform overridden on the subclass.
    // REPL (.103, make-instance 'traditional-hepburn):
    // simpl=("oo" "ō" "ou" "ō" "uu" "ū").
    assert_eq!(
        TraditionalHepburn::new().0.simplifications,
        vec!["oo", "ō", "ou", "ō", "uu", "ū"]
    );
}

#[test]
fn inherits_the_hepburn_kana_table() {
    // The kana-table is inherited unchanged from generic-hepburn (not
    // kunrei, not the modified-hepburn :wo override). REPL (.103,
    // make-instance 'traditional-hepburn): kana-count=83, :shi="shi",
    // :wo="wo", :ji="ji".
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
    // romanize.lisp:163 — simplifications initform overridden on the subclass.
    // REPL (.103, *hepburn-modified*): simpl=("oo" "ō" "ou" "ō" "uu" "ū" "aa" "ā" "ee" "ē").
    assert_eq!(
        ModifiedHepburn::new().0.simplifications,
        vec!["oo", "ō", "ou", "ō", "uu", "ū", "aa", "ā", "ee", "ē"]
    );
}

#[test]
fn wo_override_on_inherited_hepburn_table() {
    // romanize.lisp:165-166 — :after sets :wo to "o"; the rest of the
    // table is the inherited hepburn copy. REPL (.103, *hepburn-modified*):
    // kana-count=83, :wo="o", :shi="shi", :ji="ji", :wi="wi".
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
    // modified-hepburn inherits simplified-hepburn's r-simplify (n' drop +
    // simplifications fold), NOT traditional's n->m rule. REPL (.103,
    // (r-simplify *hepburn-modified* X)), 2026-05-25.
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
    // romanize.lisp:195 — kana-table is a copy-hash-table of
    // *kunrei-siki-kana-table*. REPL (.103, make-instance 'kunrei-siki):
    // kana-count=83, :shi="si", :ji="zi", :fu="hu", :wo="o", :wi="i", :we="e".
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
    // romanize.lisp:203 — defvar bound to *hepburn-traditional*; the two
    // are EQ. REPL (.103): (eq *default-romanization-method* *hepburn-traditional*) => T.
    assert!(std::ptr::eq(
        default_romanization_method(),
        hepburn_traditional()
    ));
}
