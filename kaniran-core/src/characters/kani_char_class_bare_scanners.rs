//! Rust-only sidecar within the `ichiran/characters` port module.
//! Has no Lisp counterpart.
//!
//! Lazily-compiled bare scanners — one [`fancy_regex::Regex`] per
//! [`CharClass`] using the pattern from `*char-class-regex-mapping*`
//! directly (no `+` repetition wrapper). Used by
//! [`super::char_classes::count_char_class`] for single-character
//! matches. The repeated form `(?:pat)+` is in
//! [`super::char_classes::char_scanners_inner`]; the anchored
//! form `^pat+$` is in [`super::char_classes::char_scanners`].
//!
//! Upstream recompiles the regex per call (the Lisp uses the raw
//! pattern string in `ppcre:do-matches` each time). The cache is a
//! Rust-only optimization — semantics are identical.
//!
//! Naming convention: `kani_<snake_name>.rs` marks a Rust-only
//! sidecar (no Lisp counterpart).

use std::collections::HashMap;
use std::sync::OnceLock;

use fancy_regex::Regex;

use super::char_classes::CHAR_CLASS_REGEX_MAPPING;
use super::char_classes::CharClass;

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
