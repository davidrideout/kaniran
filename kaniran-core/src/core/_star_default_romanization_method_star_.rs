//! Port of `ichiran:*default-romanization-method*` (`romanize.lisp:203`).
//!
//! The default romanization method — the same instance as
//! `*hepburn-traditional*`.

use super::_star_hepburn_traditional_star_::hepburn_traditional;
use super::traditional_hepburn_class::TraditionalHepburn;

pub fn default_romanization_method() -> &'static TraditionalHepburn {
    hepburn_traditional()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_the_same_instance_as_hepburn_traditional() {
        // romanize.lisp:203 — defvar bound to *hepburn-traditional*; the two
        // are EQ. REPL (.103): (eq *default-romanization-method* *hepburn-traditional*) => T.
        assert!(std::ptr::eq(
            default_romanization_method(),
            hepburn_traditional()
        ));
    }
}
