use super::helpers::{hepburn_kana_table, kunrei_siki_kana_table};
use crate::characters::char_class::simplify_ngrams;
use crate::characters::kani_kana_class::KanaClass;
use fancy_regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Port of `ichiran:generic-romanization` (`romanize.lisp:62`).
///
/// Base romanization method carrying a `kana-table` that maps each kana
/// mora class to its Latin spelling.
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
/// per instantiated `generic-romanization` subclass (`romanize.lisp:62-201`).
#[derive(Debug, Clone, Copy)]
pub enum RomanizationMethod<'a> {
    GenericHepburn(&'a GenericHepburn),
    SimplifiedHepburn(&'a SimplifiedHepburn),
    TraditionalHepburn(&'a TraditionalHepburn),
    ModifiedHepburn(&'a ModifiedHepburn),
    KunreiSiki(&'a KunreiSiki),
}

impl RomanizationMethod<'_> {
    /// Port of the `kana-table` `:reader` (`romanize.lisp:63`) — the table
    /// `r-base` / `r-apply` look mora and modifier classes up in.
    pub fn kana_table(&self) -> &HashMap<KanaClass, &'static str> {
        match self {
            RomanizationMethod::GenericHepburn(method) => &method.0.kana_table,
            RomanizationMethod::SimplifiedHepburn(method) => &method.base.0.kana_table,
            RomanizationMethod::TraditionalHepburn(method) => &method.0.base.0.kana_table,
            RomanizationMethod::ModifiedHepburn(method) => &method.0.base.0.kana_table,
            RomanizationMethod::KunreiSiki(method) => &method.0.kana_table,
        }
    }
}

/// Port of `ichiran:generic-hepburn` (`romanize.lisp:103`).
///
/// Romanization method whose kana-table is a copy of
/// `*hepburn-kana-table*`.
#[derive(Debug, Clone)]
pub struct GenericHepburn(pub GenericRomanization);

impl GenericHepburn {
    pub fn new() -> Self {
        // romanize.lisp:104 — (kana-table :initform (copy-hash-table *hepburn-kana-table*))
        GenericHepburn(GenericRomanization {
            kana_table: hepburn_kana_table().clone(),
        })
    }

    /// `r-simplify` method (`romanize.lisp:132-134`): drop the apostrophe
    /// after `n` when the following character is not a vowel or `y`.
    pub fn r_simplify(&self, str: &str) -> String {
        generic_hepburn_class_n_apos_consonant()
            .replace_all(str, "n${1}")
            .into_owned()
    }
}

/// `n'([^aiueoy]|$)` — also inlined by `kunrei-siki`'s `r-simplify`
/// (`romanize.lisp:198`), which cannot reach this method via call-next.
fn generic_hepburn_class_n_apos_consonant() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("n'([^aiueoy]|$)").expect("n-apostrophe scanner compiles"))
}

/// Port of `ichiran:simplified-hepburn` (`romanize.lisp:136`).
///
/// Hepburn variant carrying a `simplifications` list of alternating
/// from/to spellings (e.g. `("ou" "o" "uu" "u")`).
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

    /// `r-simplify` method (`romanize.lisp:141-142`): run the generic-hepburn
    /// simplification (`call-next-method`), then fold the `simplifications`
    /// slot's from/to pairs through `simplify-ngrams`. The slot is the flat
    /// alternating list `simplify-ngrams` itself pairs up by `cddr`
    /// (`characters.lisp:211`).
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

/// Port of `ichiran:*hepburn-basic*` (`romanize.lisp:144`).
pub fn hepburn_basic() -> &'static GenericHepburn {
    static CACHE: OnceLock<GenericHepburn> = OnceLock::new();
    CACHE.get_or_init(GenericHepburn::new)
}

/// Port of `ichiran:*hepburn-simple*` (`romanize.lisp:146-147`).
pub fn hepburn_simple() -> &'static SimplifiedHepburn {
    static CACHE: OnceLock<SimplifiedHepburn> = OnceLock::new();
    CACHE.get_or_init(|| SimplifiedHepburn::new(vec!["oo", "o", "ou", "o", "uu", "u"]))
}

/// Port of `ichiran:*hepburn-passport*` (`romanize.lisp:149-150`).
pub fn hepburn_passport() -> &'static SimplifiedHepburn {
    static CACHE: OnceLock<SimplifiedHepburn> = OnceLock::new();
    CACHE.get_or_init(|| SimplifiedHepburn::new(vec!["oo", "oh", "ou", "oh", "uu", "u"]))
}

/// Port of `ichiran:traditional-hepburn` (`romanize.lisp:152`).
///
/// Hepburn variant with simplifications `("oo" "ō" "ou" "ō" "uu" "ū")`.
#[derive(Debug, Clone)]
pub struct TraditionalHepburn(pub SimplifiedHepburn);

impl TraditionalHepburn {
    pub fn new() -> Self {
        // romanize.lisp:153 — (simplifications :initform '("oo" "ō" "ou" "ō" "uu" "ū"))
        TraditionalHepburn(SimplifiedHepburn::new(vec![
            "oo", "ō", "ou", "ō", "uu", "ū",
        ]))
    }

    /// `r-simplify` method (`romanize.lisp:155-158`): run the simplified-hepburn
    /// simplification (`call-next-method`), then `n'` before a vowel becomes
    /// `n-`, and `n` before `m`/`b`/`p` becomes `m`.
    pub fn r_simplify(&self, str: &str) -> String {
        let str = self.0.r_simplify(str);
        let str = n_apos_vowel().replace_all(&str, "n-${1}");
        n_before_mbp().replace_all(&str, "m${1}").into_owned()
    }
}

/// `n'([aiueoy])`
fn n_apos_vowel() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("n'([aiueoy])").expect("n-apostrophe-vowel scanner compiles"))
}

/// `n([mbp])`
fn n_before_mbp() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("n([mbp])").expect("n-before-labial scanner compiles"))
}

/// Port of `ichiran:*hepburn-traditional*` (`romanize.lisp:160`).
pub fn hepburn_traditional() -> &'static TraditionalHepburn {
    static CACHE: OnceLock<TraditionalHepburn> = OnceLock::new();
    CACHE.get_or_init(TraditionalHepburn::new)
}

/// Port of `ichiran:modified-hepburn` (`romanize.lisp:162`).
///
/// Hepburn variant with simplifications
/// `("oo" "ō" "ou" "ō" "uu" "ū" "aa" "ā" "ee" "ē")` that also maps `:wo`
/// to `"o"` instead of the inherited `"wo"`.
#[derive(Debug, Clone)]
pub struct ModifiedHepburn(pub SimplifiedHepburn);

impl ModifiedHepburn {
    pub fn new() -> Self {
        // romanize.lisp:163 — (simplifications :initform '("oo" "ō" "ou" "ō" "uu" "ū" "aa" "ā" "ee" "ē"))
        let mut base =
            SimplifiedHepburn::new(vec!["oo", "ō", "ou", "ō", "uu", "ū", "aa", "ā", "ee", "ē"]);
        // romanize.lisp:165-166 (initialize-instance :after modified-hepburn) — (setf (gethash :wo kana-table) "o")
        base.base.0.kana_table.insert(KanaClass::Wo, "o");
        ModifiedHepburn(base)
    }

    /// `r-simplify` is inherited from simplified-hepburn — modified-hepburn
    /// defines no override (`romanize.lisp:141-142`).
    pub fn r_simplify(&self, str: &str) -> String {
        self.0.r_simplify(str)
    }
}

/// Port of `ichiran:*hepburn-modified*` (`romanize.lisp:168`).
pub fn hepburn_modified() -> &'static ModifiedHepburn {
    static CACHE: OnceLock<ModifiedHepburn> = OnceLock::new();
    CACHE.get_or_init(ModifiedHepburn::new)
}

/// Port of `ichiran:kunrei-siki` (`romanize.lisp:194`).
///
/// Romanization method whose kana-table is a copy of
/// `*kunrei-siki-kana-table*`.
#[derive(Debug, Clone)]
pub struct KunreiSiki(pub GenericRomanization);

impl KunreiSiki {
    pub fn new() -> Self {
        // romanize.lisp:195 — (kana-table :initform (copy-hash-table *kunrei-siki-kana-table*))
        KunreiSiki(GenericRomanization {
            kana_table: kunrei_siki_kana_table().clone(),
        })
    }

    /// `r-simplify` method (`romanize.lisp:197-199`): drop the apostrophe
    /// after `n` before a non-vowel (inlined here — kunrei-siki has no
    /// generic-hepburn ancestor to reach via call-next), then fold long
    /// vowels through `simplify-ngrams`.
    pub fn r_simplify(&self, str: &str) -> String {
        let str = kunrei_siki_class_n_apos_consonant().replace_all(str, "n${1}");
        simplify_ngrams(&str, &[("oo", "ô"), ("ou", "ô"), ("uu", "û")])
    }
}

/// `n'([^aiueoy]|$)` — same pattern generic-hepburn's `r-simplify` uses
/// (`romanize.lisp:134`); duplicated because the call-next chain does not
/// reach it from kunrei-siki.
fn kunrei_siki_class_n_apos_consonant() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("n'([^aiueoy]|$)").expect("n-apostrophe scanner compiles"))
}

/// Port of `ichiran:*kunrei-siki*` (`romanize.lisp:201`).
pub fn kunrei_siki() -> &'static KunreiSiki {
    static CACHE: OnceLock<KunreiSiki> = OnceLock::new();
    CACHE.get_or_init(KunreiSiki::new)
}

/// Port of `ichiran:*default-romanization-method*` (`romanize.lisp:203`).
///
/// The default romanization method — the same instance as
/// `*hepburn-traditional*`.
pub fn default_romanization_method() -> &'static TraditionalHepburn {
    hepburn_traditional()
}

#[cfg(test)]
mod tests;
