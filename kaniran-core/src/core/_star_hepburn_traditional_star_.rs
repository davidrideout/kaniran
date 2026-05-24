//! Port of `ichiran:*hepburn-traditional*` (`romanize.lisp:160`).

use std::sync::OnceLock;

use super::traditional_hepburn_class::TraditionalHepburn;

static CACHE: OnceLock<TraditionalHepburn> = OnceLock::new();

pub fn hepburn_traditional() -> &'static TraditionalHepburn {
    CACHE.get_or_init(TraditionalHepburn::new)
}
