//! Port of `ichiran:*hepburn-passport*` (`romanize.lisp:149-150`).

use std::sync::OnceLock;

use super::simplified_hepburn_class::SimplifiedHepburn;

static CACHE: OnceLock<SimplifiedHepburn> = OnceLock::new();

pub fn hepburn_passport() -> &'static SimplifiedHepburn {
    CACHE.get_or_init(|| SimplifiedHepburn::new(vec!["oo", "oh", "ou", "oh", "uu", "u"]))
}
