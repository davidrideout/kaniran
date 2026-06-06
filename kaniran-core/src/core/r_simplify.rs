//! Port of `ichiran:r-simplify` (gf — `romanize.lisp:57-59`, `132-199`).
//!
//! Post-processes a romanized string per the method's orthography
//! (apostrophe handling, long-vowel folding, the traditional `n-`/`m`
//! rules); the default is the identity.

use super::generic_romanization_class::RomanizationMethod;

pub fn r_simplify(method: RomanizationMethod<'_>, str: &str) -> String {
    match method {
        RomanizationMethod::GenericHepburn(method) => method.r_simplify(str),
        RomanizationMethod::SimplifiedHepburn(method) => method.r_simplify(str),
        RomanizationMethod::TraditionalHepburn(method) => method.r_simplify(str),
        RomanizationMethod::ModifiedHepburn(method) => method.r_simplify(str),
        RomanizationMethod::KunreiSiki(method) => method.r_simplify(str),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::generic_hepburn_class::GenericHepburn;
    use crate::core::kunrei_siki_class::KunreiSiki;
    use crate::core::simplified_hepburn_class::SimplifiedHepburn;
    use crate::core::traditional_hepburn_class::TraditionalHepburn;

    #[test]
    fn r_simplify_fixtures() {
        // REPL fixtures (.103, ichiran::r-simplify), 2026-05-24.
        let generic = GenericHepburn::new();
        let simple = SimplifiedHepburn::new(vec!["oo", "o", "ou", "o", "uu", "u"]);
        let passport = SimplifiedHepburn::new(vec!["oo", "oh", "ou", "oh", "uu", "u"]);
        let traditional = TraditionalHepburn::new();
        let kunrei = KunreiSiki::new();
        let hepburn = RomanizationMethod::GenericHepburn(&generic);
        let simple = RomanizationMethod::SimplifiedHepburn(&simple);
        let passport = RomanizationMethod::SimplifiedHepburn(&passport);
        let traditional = RomanizationMethod::TraditionalHepburn(&traditional);
        let kunrei = RomanizationMethod::KunreiSiki(&kunrei);
        // (label, method, input, expected).
        let cases: &[(&str, RomanizationMethod, &str, &str)] = &[
            // generic-hepburn: n' drops before a non-vowel, stays before vowel/y
            ("hepburn n'+consonant", hepburn, "kon'nichiwa", "konnichiwa"),
            ("hepburn n'+vowel", hepburn, "han'i", "han'i"),
            ("hepburn n'+y", hepburn, "shin'you", "shin'you"),
            // simplified-hepburn: call-next + simplifications slot
            ("simple long-o", simple, "koukou", "koko"),
            ("simple n'+ngram", simple, "n'pou", "npo"),
            ("passport long-o", passport, "koukou", "kohkoh"),
            // traditional-hepburn: macron fold + n-/m rules
            ("traditional long-o", traditional, "koukou", "kōkō"),
            ("traditional n+b", traditional, "shinbun", "shimbun"),
            ("traditional n'+vowel", traditional, "kon'i", "kon-i"),
            ("traditional n+m", traditional, "honma", "homma"),
            ("traditional n'+y", traditional, "shin'you", "shin-yō"),
            ("traditional n+p", traditional, "kanpeki", "kampeki"),
            // kunrei-siki: inline n' drop + circumflex long-vowel fold
            ("kunrei long-o", kunrei, "koukou", "kôkô"),
            ("kunrei n'+consonant", kunrei, "kon'nichi", "konnichi"),
        ];
        for (label, method, input, expected) in cases {
            assert_eq!(&r_simplify(*method, input), expected, "case={label}");
        }
    }
}
