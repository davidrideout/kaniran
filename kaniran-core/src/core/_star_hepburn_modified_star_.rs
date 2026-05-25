//! Port of `ichiran:*hepburn-modified*` (`romanize.lisp:168`).

use std::sync::OnceLock;

use super::modified_hepburn_class::ModifiedHepburn;

static CACHE: OnceLock<ModifiedHepburn> = OnceLock::new();

pub fn hepburn_modified() -> &'static ModifiedHepburn {
    CACHE.get_or_init(ModifiedHepburn::new)
}
