//! Rust-only sidecar within the `ichiran/characters` port module.
//! Has no Lisp counterpart.
//!
//! Bare scanners — one [`fancy_regex::Regex`] per [`CharClass`] using
//! the pattern from `*char-class-regex-mapping*` directly (no `+`
//! repetition wrapper), matching individual characters of a class
//! rather than runs of them.

use std::collections::HashMap;
use std::sync::OnceLock;

use fancy_regex::Regex;

use super::constants::CHAR_CLASS_REGEX_MAPPING;
use super::char_class::CharClass;

pub fn char_class_bare_scanners() -> &'static HashMap<CharClass, Regex> {
    static CACHE: OnceLock<HashMap<CharClass, Regex>> = OnceLock::new();
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
