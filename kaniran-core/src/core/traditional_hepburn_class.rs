//! Port of `ichiran:traditional-hepburn` (`romanize.lisp:152`).
//!
//! Subclass of simplified-hepburn. Redefines the simplifications
//! initform to `("oo" "ō" "ou" "ō" "uu" "ū")`. Adds no slots — newtype
//! over [`SimplifiedHepburn`]; the kana-table stays the inherited
//! hepburn copy.

use std::sync::OnceLock;

use fancy_regex::Regex;

use super::simplified_hepburn_class::SimplifiedHepburn;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characters::kana_class::KanaClass;

    #[test]
    fn redefined_simplifications_initform() {
        // romanize.lisp:153 — simplifications initform overridden on the subclass.
        // REPL (.103, make-instance 'traditional-hepburn):
        // simpl=("oo" "ō" "ou" "ō" "uu" "ū").
        assert_eq!(
            TraditionalHepburn::new().0.simplifications,
            vec!["oo", "ō", "ou", "ō", "uu", "ū"]
        );
    }

    #[test]
    fn inherits_the_hepburn_kana_table() {
        // The kana-table is inherited unchanged from generic-hepburn (not
        // kunrei, not the modified-hepburn :wo override). REPL (.103,
        // make-instance 'traditional-hepburn): kana-count=83, :shi="shi",
        // :wo="wo", :ji="ji".
        let method = TraditionalHepburn::new();
        let kana_table = &method.0.base.0.kana_table;
        assert_eq!(kana_table.len(), 83);
        assert_eq!(kana_table.get(&KanaClass::Shi), Some(&"shi"));
        assert_eq!(kana_table.get(&KanaClass::Wo), Some(&"wo"));
        assert_eq!(kana_table.get(&KanaClass::Ji), Some(&"ji"));
    }
}
