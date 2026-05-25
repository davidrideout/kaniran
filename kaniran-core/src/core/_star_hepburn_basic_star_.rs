//! Port of `ichiran:*hepburn-basic*` (`romanize.lisp:144`).

use std::sync::OnceLock;

use super::generic_hepburn_class::GenericHepburn;

static CACHE: OnceLock<GenericHepburn> = OnceLock::new();

pub fn hepburn_basic() -> &'static GenericHepburn {
    CACHE.get_or_init(GenericHepburn::new)
}
