//! Port of `ichiran:kunrei-siki` (`romanize.lisp:194`).
//!
//! Subclass of generic-romanization. Redefines the kana-table initform
//! to a copy of `*kunrei-siki-kana-table*`. Adds no slots — newtype
//! over [`GenericRomanization`].

use std::sync::OnceLock;

use fancy_regex::Regex;

use super::_star_kunrei_siki_kana_table_star_::kunrei_siki_kana_table;
use super::generic_romanization_class::GenericRomanization;
use crate::characters::normalize::simplify_ngrams;

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
        let str = n_apos_consonant().replace_all(str, "n${1}");
        simplify_ngrams(&str, &[("oo", "ô"), ("ou", "ô"), ("uu", "û")])
    }
}

/// `n'([^aiueoy]|$)` — same pattern generic-hepburn's `r-simplify` uses
/// (`romanize.lisp:134`); duplicated because the call-next chain does not
/// reach it from kunrei-siki.
fn n_apos_consonant() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("n'([^aiueoy]|$)").expect("n-apostrophe scanner compiles"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characters::kana_class::KanaClass;

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
}
