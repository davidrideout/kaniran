//! Port of `ichiran:simplified-hepburn` (`romanize.lisp:136`).
//!
//! Subclass of generic-hepburn. Adds a `simplifications` slot
//! (`:initarg :simplifications`, `:initform nil`) — a flat list of
//! alternating from/to spellings (e.g. `("ou" "o" "uu" "u")`). The
//! `:initform nil` maps to an empty list.

use super::generic_hepburn_class::GenericHepburn;

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
