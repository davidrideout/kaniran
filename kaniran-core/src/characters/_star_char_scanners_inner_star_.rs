//! Port of `ichiran/characters:*char-scanners-inner*`
//! (`characters.lisp:155`).
//!
//! Like [`super::_star_char_scanners_star_`] but unanchored: each
//! pattern is wrapped as `(?:pattern)+`, matching one or more
//! characters of the class.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::_star_char_class_regex_mapping_star_::CHAR_CLASS_REGEX_MAPPING;
use super::char_class_type::CharClass;

static CACHE: OnceLock<HashMap<CharClass, fancy_regex::Regex>> = OnceLock::new();

pub fn char_scanners_inner() -> &'static HashMap<CharClass, fancy_regex::Regex> {
    CACHE.get_or_init(|| {
        let mut h = HashMap::new();
        for (class, pat) in CHAR_CLASS_REGEX_MAPPING {
            let wrapped = format!("(?:{pat})+");
            let re = fancy_regex::Regex::new(&wrapped)
                .unwrap_or_else(|e| panic!("class {class:?} pattern {wrapped:?} failed: {e}"));
            h.insert(*class, re);
        }
        h
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_under_fancy_regex() {
        let h = char_scanners_inner();
        for (class, _) in CHAR_CLASS_REGEX_MAPPING {
            assert!(h.contains_key(class), "missing scanner for {class:?}");
        }
    }
}
