//! Port of `ichiran:kunrei-siki` (`romanize.lisp:194`).
//!
//! Subclass of generic-romanization. Redefines the kana-table initform
//! to a copy of `*kunrei-siki-kana-table*`. Adds no slots — newtype
//! over [`GenericRomanization`].

use super::_star_kunrei_siki_kana_table_star_::kunrei_siki_kana_table;
use super::generic_romanization_class::GenericRomanization;

#[derive(Debug, Clone)]
pub struct KunreiSiki(pub GenericRomanization);

impl KunreiSiki {
    pub fn new() -> Self {
        // romanize.lisp:195 — (kana-table :initform (copy-hash-table *kunrei-siki-kana-table*))
        KunreiSiki(GenericRomanization {
            kana_table: kunrei_siki_kana_table().clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characters::kani_kana_class::KanaClass;

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
