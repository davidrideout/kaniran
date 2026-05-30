//! Romanization methods: class hierarchy, sidecars, and method
//! instances. From `romanize.lisp:62-203`.

use std::collections::HashMap;
use std::sync::OnceLock;

use fancy_regex::Regex;

use super::tables::{hepburn_kana_table, kunrei_siki_kana_table};
use crate::characters::kana_class::KanaClass;
use crate::characters::normalize::simplify_ngrams;

// -- character-class tree (rust-only sidecars) ---------------------------

/// One element of a character-class list — per-character result of
/// looking a glyph up in `*char-class-hash*` with the default-as-self
/// idiom. Upstream uses a keyword-or-character cons element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CcItem {
    Class(KanaClass),
    Char(char),
}

/// One node of the tree `process-modifiers` builds and `romanize-core`
/// / `leftmost-atom` walk. Upstream uses bare cons cells:
/// `Atom` (keyword or char), `Nil` (empty modifier slot), or
/// `Node(head, tail)` for `(:+ya <child>)` / `(:sokuon . <rest>)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CcTree {
    Nil,
    Atom(CcItem),
    Node(KanaClass, Vec<CcTree>),
}

// -- class hierarchy (romanize.lisp:62-200) -------------------------------

/// `generic-romanization` (`romanize.lisp:62`). Base romanization
/// method. Carries the `kana-table` mapping mora class → Latin
/// spelling. Subclasses redefine the initform.
#[derive(Debug, Clone)]
pub struct GenericRomanization {
    pub kana_table: HashMap<KanaClass, &'static str>,
}

impl GenericRomanization {
    pub fn new() -> Self {
        GenericRomanization {
            kana_table: HashMap::new(),
        }
    }
}

/// The `method` argument the `r-*` generics dispatch on — one variant
/// per instantiated `generic-romanization` subclass
/// (`romanize.lisp:62-201`).
#[derive(Debug, Clone, Copy)]
pub enum RomanizationMethod<'a> {
    GenericHepburn(&'a GenericHepburn),
    SimplifiedHepburn(&'a SimplifiedHepburn),
    TraditionalHepburn(&'a TraditionalHepburn),
    ModifiedHepburn(&'a ModifiedHepburn),
    KunreiSiki(&'a KunreiSiki),
}

impl RomanizationMethod<'_> {
    /// `kana-table` `:reader` (`romanize.lisp:63`).
    pub fn kana_table(&self) -> &HashMap<KanaClass, &'static str> {
        match self {
            RomanizationMethod::GenericHepburn(m) => &m.0.kana_table,
            RomanizationMethod::SimplifiedHepburn(m) => &m.base.0.kana_table,
            RomanizationMethod::TraditionalHepburn(m) => &m.0.base.0.kana_table,
            RomanizationMethod::ModifiedHepburn(m) => &m.0.base.0.kana_table,
            RomanizationMethod::KunreiSiki(m) => &m.0.kana_table,
        }
    }
}

/// Upstream the `method` slot holds either a `generic-romanization`
/// instance or the keyword `:kana`; this enum adds the `:kana` arm so
/// the entry points can dispatch on it.
#[derive(Debug, Clone, Copy)]
pub enum KaniRomanizeMethod<'a> {
    Kana,
    Method(RomanizationMethod<'a>),
}

/// `generic-hepburn` (`romanize.lisp:103`). Subclass that copies
/// `*hepburn-kana-table*` into its `kana-table`. Newtype over
/// [`GenericRomanization`] — adds no slots.
#[derive(Debug, Clone)]
pub struct GenericHepburn(pub GenericRomanization);

impl GenericHepburn {
    pub fn new() -> Self {
        GenericHepburn(GenericRomanization {
            kana_table: hepburn_kana_table().clone(),
        })
    }

    /// `r-simplify` method (`romanize.lisp:132-134`): drop the
    /// apostrophe after `n` when the following character is not a
    /// vowel or `y`.
    pub fn r_simplify(&self, str: &str) -> String {
        n_apos_consonant().replace_all(str, "n${1}").into_owned()
    }
}

/// `n'([^aiueoy]|$)` — also inlined by `kunrei-siki`'s `r-simplify`
/// (`romanize.lisp:198`), which cannot reach this method via
/// call-next.
fn n_apos_consonant() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("n'([^aiueoy]|$)").expect("n-apostrophe scanner compiles"))
}

/// `simplified-hepburn` (`romanize.lisp:136`). Adds a `simplifications`
/// slot — a flat list of alternating from/to spellings.
#[derive(Debug, Clone)]
pub struct SimplifiedHepburn {
    pub base: GenericHepburn,
    pub simplifications: Vec<&'static str>,
}

impl SimplifiedHepburn {
    pub fn new(simplifications: Vec<&'static str>) -> Self {
        SimplifiedHepburn {
            base: GenericHepburn::new(),
            simplifications,
        }
    }

    /// `r-simplify` method (`romanize.lisp:141-142`): generic-hepburn's
    /// `r-simplify` (via `call-next-method`), then fold the
    /// `simplifications` slot's from/to pairs through `simplify-ngrams`.
    pub fn r_simplify(&self, str: &str) -> String {
        let str = self.base.r_simplify(str);
        let pairs: Vec<(&str, &str)> = self
            .simplifications
            .chunks(2)
            .filter(|pair| pair.len() == 2)
            .map(|pair| (pair[0], pair[1]))
            .collect();
        simplify_ngrams(&str, &pairs)
    }
}

/// `traditional-hepburn` (`romanize.lisp:152`). Subclass of
/// simplified-hepburn with `simplifications` initform
/// `("oo" "ō" "ou" "ō" "uu" "ū")`. Newtype — no added slots.
#[derive(Debug, Clone)]
pub struct TraditionalHepburn(pub SimplifiedHepburn);

impl TraditionalHepburn {
    pub fn new() -> Self {
        TraditionalHepburn(SimplifiedHepburn::new(vec![
            "oo", "ō", "ou", "ō", "uu", "ū",
        ]))
    }

    /// `r-simplify` method (`romanize.lisp:155-158`): simplified-hepburn
    /// (`call-next-method`), then `n'` before a vowel becomes `n-`, and
    /// `n` before `m`/`b`/`p` becomes `m`.
    pub fn r_simplify(&self, str: &str) -> String {
        let str = self.0.r_simplify(str);
        let str = n_apos_vowel().replace_all(&str, "n-${1}");
        n_before_mbp().replace_all(&str, "m${1}").into_owned()
    }
}

fn n_apos_vowel() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("n'([aiueoy])").expect("n-apostrophe-vowel scanner compiles"))
}

fn n_before_mbp() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("n([mbp])").expect("n-before-labial scanner compiles"))
}

/// `modified-hepburn` (`romanize.lisp:162`). Subclass of
/// simplified-hepburn with `simplifications` initform
/// `("oo" "ō" "ou" "ō" "uu" "ū" "aa" "ā" "ee" "ē")` and an
/// `initialize-instance :after` that overrides the kana-table `:wo`
/// entry to `"o"`. Newtype — no added slots. `r-simplify` inherited.
#[derive(Debug, Clone)]
pub struct ModifiedHepburn(pub SimplifiedHepburn);

impl ModifiedHepburn {
    pub fn new() -> Self {
        let mut base = SimplifiedHepburn::new(vec![
            "oo", "ō", "ou", "ō", "uu", "ū", "aa", "ā", "ee", "ē",
        ]);
        // romanize.lisp:165-166 (initialize-instance :after) — (setf (gethash :wo kana-table) "o")
        base.base.0.kana_table.insert(KanaClass::Wo, "o");
        ModifiedHepburn(base)
    }

    /// `r-simplify` inherited from simplified-hepburn — no override on
    /// this subclass (`romanize.lisp:141-142`).
    pub fn r_simplify(&self, str: &str) -> String {
        self.0.r_simplify(str)
    }
}

/// `kunrei-siki` (`romanize.lisp:194`). Subclass of
/// generic-romanization that copies `*kunrei-siki-kana-table*` into its
/// `kana-table`. Newtype — no added slots.
#[derive(Debug, Clone)]
pub struct KunreiSiki(pub GenericRomanization);

impl KunreiSiki {
    pub fn new() -> Self {
        KunreiSiki(GenericRomanization {
            kana_table: kunrei_siki_kana_table().clone(),
        })
    }

    /// `r-simplify` method (`romanize.lisp:197-199`): drop the
    /// apostrophe after `n` before a non-vowel (inlined — kunrei-siki
    /// has no generic-hepburn ancestor to reach via call-next), then
    /// fold long vowels.
    pub fn r_simplify(&self, str: &str) -> String {
        let str = n_apos_consonant().replace_all(str, "n${1}");
        simplify_ngrams(&str, &[("oo", "ô"), ("ou", "ô"), ("uu", "û")])
    }
}

// -- method instances (romanize.lisp:144-203) -----------------------------

/// `*hepburn-basic*` (`romanize.lisp:144`).
pub fn hepburn_basic() -> &'static GenericHepburn {
    static CACHE: OnceLock<GenericHepburn> = OnceLock::new();
    CACHE.get_or_init(GenericHepburn::new)
}

/// `*hepburn-simple*` (`romanize.lisp:146-147`).
pub fn hepburn_simple() -> &'static SimplifiedHepburn {
    static CACHE: OnceLock<SimplifiedHepburn> = OnceLock::new();
    CACHE.get_or_init(|| SimplifiedHepburn::new(vec!["oo", "o", "ou", "o", "uu", "u"]))
}

/// `*hepburn-passport*` (`romanize.lisp:149-150`).
pub fn hepburn_passport() -> &'static SimplifiedHepburn {
    static CACHE: OnceLock<SimplifiedHepburn> = OnceLock::new();
    CACHE.get_or_init(|| SimplifiedHepburn::new(vec!["oo", "oh", "ou", "oh", "uu", "u"]))
}

/// `*hepburn-traditional*` (`romanize.lisp:160`).
pub fn hepburn_traditional() -> &'static TraditionalHepburn {
    static CACHE: OnceLock<TraditionalHepburn> = OnceLock::new();
    CACHE.get_or_init(TraditionalHepburn::new)
}

/// `*hepburn-modified*` (`romanize.lisp:168`).
pub fn hepburn_modified() -> &'static ModifiedHepburn {
    static CACHE: OnceLock<ModifiedHepburn> = OnceLock::new();
    CACHE.get_or_init(ModifiedHepburn::new)
}

/// `*kunrei-siki*` (`romanize.lisp:201`).
pub fn kunrei_siki() -> &'static KunreiSiki {
    static CACHE: OnceLock<KunreiSiki> = OnceLock::new();
    CACHE.get_or_init(KunreiSiki::new)
}

/// `*default-romanization-method*` (`romanize.lisp:203`). Defvar bound
/// to `*hepburn-traditional*` — the two are `eq`.
pub fn default_romanization_method() -> &'static TraditionalHepburn {
    hepburn_traditional()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_kana_table_is_empty() {
        assert_eq!(GenericRomanization::new().kana_table.len(), 0);
    }

    /// REPL (.103, make-instance 'generic-hepburn): kana-count=83, :shi="shi".
    #[test]
    fn generic_hepburn_carries_a_copy_of_the_hepburn_table() {
        let method = GenericHepburn::new();
        assert_eq!(method.0.kana_table.len(), 83);
        assert_eq!(method.0.kana_table.get(&KanaClass::Shi), Some(&"shi"));
    }

    #[test]
    fn simplified_hepburn_default_simplifications_is_empty() {
        let method = SimplifiedHepburn::new(Vec::new());
        assert!(method.simplifications.is_empty());
        assert_eq!(method.base.0.kana_table.len(), 83);
    }

    #[test]
    fn simplified_hepburn_initarg_simplifications_pass_through() {
        let method = SimplifiedHepburn::new(vec!["xx", "y"]);
        assert_eq!(method.simplifications, vec!["xx", "y"]);
    }

    /// REPL (.103, make-instance 'traditional-hepburn): kana-count=83, :shi="shi", :wo="wo", :ji="ji".
    #[test]
    fn traditional_hepburn_inherits_the_hepburn_kana_table() {
        let method = TraditionalHepburn::new();
        let kana_table = &method.0.base.0.kana_table;
        assert_eq!(kana_table.len(), 83);
        assert_eq!(kana_table.get(&KanaClass::Shi), Some(&"shi"));
        assert_eq!(kana_table.get(&KanaClass::Wo), Some(&"wo"));
        assert_eq!(kana_table.get(&KanaClass::Ji), Some(&"ji"));
    }

    #[test]
    fn traditional_hepburn_redefined_simplifications_initform() {
        assert_eq!(
            TraditionalHepburn::new().0.simplifications,
            vec!["oo", "ō", "ou", "ō", "uu", "ū"]
        );
    }

    /// REPL (.103, *hepburn-modified*): kana-count=83, :wo="o", :shi="shi", :ji="ji", :wi="wi".
    #[test]
    fn modified_hepburn_wo_override_on_inherited_table() {
        let method = ModifiedHepburn::new();
        let kana_table = &method.0.base.0.kana_table;
        assert_eq!(kana_table.len(), 83);
        assert_eq!(kana_table.get(&KanaClass::Wo), Some(&"o"));
        assert_eq!(kana_table.get(&KanaClass::Shi), Some(&"shi"));
        assert_eq!(kana_table.get(&KanaClass::Ji), Some(&"ji"));
        assert_eq!(kana_table.get(&KanaClass::Wi), Some(&"wi"));
    }

    #[test]
    fn modified_hepburn_redefined_simplifications_initform() {
        assert_eq!(
            ModifiedHepburn::new().0.simplifications,
            vec!["oo", "ō", "ou", "ō", "uu", "ū", "aa", "ā", "ee", "ē"]
        );
    }

    /// modified-hepburn inherits simplified-hepburn's r-simplify (n'
    /// drop + simplifications fold), NOT traditional's n→m rule. REPL
    /// (.103), 2026-05-25.
    #[test]
    fn modified_hepburn_r_simplify_inherits_simplified() {
        let method = ModifiedHepburn::new();
        let cases: &[(&str, &str)] = &[
            ("koukou", "kōkō"),
            ("okaasan", "okāsan"),
            ("oneesan", "onēsan"),
            ("suugaku", "sūgaku"),
            ("kon'nichiwa", "konnichiwa"),
            ("han'i", "han'i"),
            ("shinbun", "shinbun"),
            ("honma", "honma"),
        ];
        for (input, expected) in cases {
            assert_eq!(&method.r_simplify(input), expected, "input={input}");
        }
    }

    /// REPL (.103, make-instance 'kunrei-siki): kana-count=83, :shi="si", :ji="zi", :fu="hu", :wo="o", :wi="i", :we="e".
    #[test]
    fn kunrei_siki_carries_a_copy_of_the_kunrei_table() {
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

    /// REPL (.103): (eq *default-romanization-method* *hepburn-traditional*) => T.
    #[test]
    fn default_method_is_the_same_instance_as_hepburn_traditional() {
        assert!(std::ptr::eq(
            default_romanization_method(),
            hepburn_traditional()
        ));
    }
}
