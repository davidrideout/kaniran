//! Rust-only sidecar within the `ichiran/characters` port module.
//! Has no Lisp counterpart.
//!
//! Lazily-compiled bare scanners — one [`fancy_regex::Regex`] per
//! [`CharClass`] using the pattern from `*char-class-regex-mapping*`
//! directly (no `+` repetition wrapper). Used by
//! [`super::count_char_class`] and [`super::collect_char_class`], which
//! find individual matches of one character of the class — not runs of
//! them. The repeated form `(?:pat)+` is in
//! [`super::_star_char_scanners_inner_star_`]; the anchored form
//! `^pat+$` is in [`super::_star_char_scanners_star_`].
//!
//! Upstream recompiles the regex per call (the Lisp uses the raw
//! pattern string in `ppcre:do-matches` each time). The cache is a
//! Rust-only optimization — semantics are identical.
//!
//! Naming convention: `kani_<snake_name>.rs` marks a Rust-only
//! sidecar — see the module-doc on [`crate::kani::naming`].

use std::collections::HashMap;
use std::sync::OnceLock;

use fancy_regex::Regex;

use super::_star_char_class_regex_mapping_star_::CHAR_CLASS_REGEX_MAPPING;
use super::char_class_type::CharClass;

static CACHE: OnceLock<HashMap<CharClass, Regex>> = OnceLock::new();

pub fn char_class_bare_scanners() -> &'static HashMap<CharClass, Regex> {
    CACHE.get_or_init(|| {
        CHAR_CLASS_REGEX_MAPPING
            .iter()
            .map(|(c, pat)| {
                let re = Regex::new(pat)
                    .unwrap_or_else(|e| panic!("class {c:?} pattern {pat:?} failed: {e}"));
                (*c, re)
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_every_class_in_the_mapping() {
        let h = char_class_bare_scanners();
        for (class, _) in CHAR_CLASS_REGEX_MAPPING {
            assert!(h.contains_key(class), "missing scanner for {class:?}");
        }
    }
}
