//! Port of `ichiran:generic-romanization` (`romanize.lisp:62`).
//!
//! Base romanization method. Carries a `kana-table` mapping each kana
//! mora class to its Latin spelling; the base initform builds an empty
//! table (`(make-hash-table)`). Subclasses redefine the initform.

use std::collections::HashMap;

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
