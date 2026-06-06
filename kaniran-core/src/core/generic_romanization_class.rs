//! Port of `ichiran:generic-romanization` (`romanize.lisp:62`).
//!
//! Base romanization method carrying a `kana-table` that maps each kana
//! mora class to its Latin spelling.

use std::collections::HashMap;

use super::generic_hepburn_class::GenericHepburn;
use super::kunrei_siki_class::KunreiSiki;
use super::modified_hepburn_class::ModifiedHepburn;
use super::simplified_hepburn_class::SimplifiedHepburn;
use super::traditional_hepburn_class::TraditionalHepburn;
use crate::characters::kani_kana_class::KanaClass;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_kana_table_is_empty() {
        // romanize.lisp:63-64 — (kana-table :initform (make-hash-table)).
        // REPL (.103, make-instance 'generic-romanization): kana-count=0.
        assert_eq!(GenericRomanization::new().kana_table.len(), 0);
    }
}
