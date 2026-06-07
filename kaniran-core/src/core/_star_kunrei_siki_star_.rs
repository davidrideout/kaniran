//! Port of `ichiran:*kunrei-siki*` (`romanize.lisp:201`).

use std::sync::OnceLock;

use super::kunrei_siki_class::KunreiSiki;

pub fn kunrei_siki() -> &'static KunreiSiki {
    static CACHE: OnceLock<KunreiSiki> = OnceLock::new();
    CACHE.get_or_init(KunreiSiki::new)
}
