//! Port of `ichiran:generic-hepburn` (`romanize.lisp:103`).
//!
//! Subclass of generic-romanization. Redefines the kana-table initform
//! to a copy of `*hepburn-kana-table*`. Adds no slots — newtype over
//! [`GenericRomanization`].

use std::sync::OnceLock;

use fancy_regex::Regex;

use super::_star_hepburn_kana_table_star_::hepburn_kana_table;
use super::generic_romanization_class::GenericRomanization;

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
        n_apos_consonant().replace_all(str, "n${1}").into_owned()
    }
}

/// `n'([^aiueoy]|$)` — also inlined by `kunrei-siki`'s `r-simplify`
/// (`romanize.lisp:198`), which cannot reach this method via call-next.
fn n_apos_consonant() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("n'([^aiueoy]|$)").expect("n-apostrophe scanner compiles"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characters::kana_class::KanaClass;

    #[test]
    fn carries_a_copy_of_the_hepburn_table() {
        // romanize.lisp:104 — kana-table is a copy-hash-table of *hepburn-kana-table*.
        // REPL (.103, make-instance 'generic-hepburn): kana-count=83, :shi="shi".
        let method = GenericHepburn::new();
        assert_eq!(method.0.kana_table.len(), 83);
        assert_eq!(method.0.kana_table.get(&KanaClass::Shi), Some(&"shi"));
    }
}
