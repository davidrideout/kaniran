use super::*;

// --- _star_char_class_regex_mapping_star_ ---
#[test]
fn every_pattern_compiles_under_fancy_regex() {
    for (class, pat) in CHAR_CLASS_REGEX_MAPPING {
        fancy_regex::Regex::new(pat)
            .unwrap_or_else(|e| panic!("class {class:?} regex {pat:?} failed: {e}"));
    }
}
