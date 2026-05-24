//! Port of `ichiran:*hepburn-simple*` (`romanize.lisp:146-147`).

use std::sync::OnceLock;

use super::simplified_hepburn_class::SimplifiedHepburn;

static CACHE: OnceLock<SimplifiedHepburn> = OnceLock::new();

pub fn hepburn_simple() -> &'static SimplifiedHepburn {
    CACHE.get_or_init(|| SimplifiedHepburn::new(vec!["oo", "o", "ou", "o", "uu", "u"]))
}
