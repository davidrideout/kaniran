//! Port of `ichiran:modified-hepburn` (`romanize.lisp:162`).
//!
//! Subclass of simplified-hepburn. Redefines the simplifications initform
//! to `("oo" "ō" "ou" "ō" "uu" "ū" "aa" "ā" "ee" "ē")` and, via an
//! `initialize-instance :after`, overrides the kana-table `:wo` entry to
//! `"o"` (the inherited hepburn copy maps `:wo` to `"wo"`). Adds no slots —
//! newtype over [`SimplifiedHepburn`]; `r-simplify` is inherited unchanged
//! from simplified-hepburn (no override on this subclass).

use super::simplified_hepburn_class::SimplifiedHepburn;
use crate::characters::kana_class::KanaClass;

#[derive(Debug, Clone)]
pub struct ModifiedHepburn(pub SimplifiedHepburn);

impl ModifiedHepburn {
    pub fn new() -> Self {
        // romanize.lisp:163 — (simplifications :initform '("oo" "ō" "ou" "ō" "uu" "ū" "aa" "ā" "ee" "ē"))
        let mut base = SimplifiedHepburn::new(vec![
            "oo", "ō", "ou", "ō", "uu", "ū", "aa", "ā", "ee", "ē",
        ]);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redefined_simplifications_initform() {
        // romanize.lisp:163 — simplifications initform overridden on the subclass.
        // REPL (.103, *hepburn-modified*): simpl=("oo" "ō" "ou" "ō" "uu" "ū" "aa" "ā" "ee" "ē").
        assert_eq!(
            ModifiedHepburn::new().0.simplifications,
            vec!["oo", "ō", "ou", "ō", "uu", "ū", "aa", "ā", "ee", "ē"]
        );
    }

    #[test]
    fn wo_override_on_inherited_hepburn_table() {
        // romanize.lisp:165-166 — :after sets :wo to "o"; the rest of the
        // table is the inherited hepburn copy. REPL (.103, *hepburn-modified*):
        // kana-count=83, :wo="o", :shi="shi", :ji="ji", :wi="wi".
        let method = ModifiedHepburn::new();
        let kana_table = &method.0.base.0.kana_table;
        assert_eq!(kana_table.len(), 83);
        assert_eq!(kana_table.get(&KanaClass::Wo), Some(&"o"));
        assert_eq!(kana_table.get(&KanaClass::Shi), Some(&"shi"));
        assert_eq!(kana_table.get(&KanaClass::Ji), Some(&"ji"));
        assert_eq!(kana_table.get(&KanaClass::Wi), Some(&"wi"));
    }

    #[test]
    fn r_simplify_inherits_simplified_hepburn() {
        // modified-hepburn inherits simplified-hepburn's r-simplify (n' drop +
        // simplifications fold), NOT traditional's n->m rule. REPL (.103,
        // (r-simplify *hepburn-modified* X)), 2026-05-25.
        let method = ModifiedHepburn::new();
        let cases: &[(&str, &str)] = &[
            ("koukou", "kōkō"),
            ("okaasan", "okāsan"),
            ("oneesan", "onēsan"),
            ("suugaku", "sūgaku"),
            ("kon'nichiwa", "konnichiwa"),
            ("han'i", "han'i"),
            // no n->m fold (that is traditional-only)
            ("shinbun", "shinbun"),
            ("honma", "honma"),
        ];
        for (input, expected) in cases {
            assert_eq!(&method.r_simplify(input), expected, "input={input}");
        }
    }
}
