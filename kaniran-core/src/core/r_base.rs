//! Port of `ichiran:r-base` (gf — `romanize.lisp:39-42`, `66-67`).
//!
//! Romanizes one atomic mora class by looking it up in the method's
//! kana-table, falling back to the downcased class name.

use super::generic_romanization_class::RomanizationMethod;
use crate::characters::kani_kana_class::KanaClass;

pub fn r_base(method: RomanizationMethod<'_>, item: KanaClass) -> String {
    match method.kana_table().get(&item) {
        // romanize.lisp:66-67 (r-base generic-romanization)
        Some(latin) => (*latin).to_string(),
        // romanize.lisp:41-42 (r-base default — string-downcase)
        None => item.lisp_name().to_ascii_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::generic_hepburn_class::GenericHepburn;
    use crate::core::generic_romanization_class::GenericRomanization;
    use crate::core::kunrei_siki_class::KunreiSiki;

    #[test]
    fn r_base_fixtures() {
        use KanaClass::*;
        // REPL fixtures (.103, ichiran::r-base), 2026-05-24.
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
        // REPL fixtures (.103, (r-base (make-instance 'generic-romanization) X)),
        // 2026-05-24 — the empty kana-table misses, so r-base downcases the
        // keyword name. Reproduced with a cleared table since the dispatcher
        // exposes only the four populated subclasses.
        let mut empty = GenericHepburn::new();
        empty.0 = GenericRomanization::new();
        let method = RomanizationMethod::GenericHepburn(&empty);
        assert_eq!(r_base(method, N), "n");
        assert_eq!(r_base(method, Ka), "ka");
        assert_eq!(r_base(method, PlusYa), "+ya");
    }
}
