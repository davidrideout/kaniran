//! Port of `ichiran:simplified-hepburn` (`romanize.lisp:136`).
//!
//! Hepburn variant carrying a `simplifications` list of alternating
//! from/to spellings (e.g. `("ou" "o" "uu" "u")`).

use super::generic_hepburn_class::GenericHepburn;
use crate::characters::char_class::simplify_ngrams;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
