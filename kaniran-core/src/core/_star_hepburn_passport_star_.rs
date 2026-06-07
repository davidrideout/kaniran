//! Port of `ichiran:*hepburn-passport*` (`romanize.lisp:149-150`).

use std::sync::OnceLock;

use super::simplified_hepburn_class::SimplifiedHepburn;

pub fn hepburn_passport() -> &'static SimplifiedHepburn {
    static CACHE: OnceLock<SimplifiedHepburn> = OnceLock::new();
    CACHE.get_or_init(|| SimplifiedHepburn::new(vec!["oo", "oh", "ou", "oh", "uu", "u"]))
}
