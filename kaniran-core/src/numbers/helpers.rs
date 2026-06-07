use super::constants::CHAR_NUMBER_CLASS;
use super::kani_num_class::NumClass;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Port of `ichiran/numbers:*char-number-class-hash*` (`numbers.lisp:18`).
///
/// Per-character lookup from any numeric glyph to its `(NumClass, u8)`
/// classification.
pub fn char_number_class_hash() -> &'static HashMap<char, (NumClass, u8)> {
    static CACHE: OnceLock<HashMap<char, (NumClass, u8)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut h = HashMap::new();
        for &(chars, class, val) in CHAR_NUMBER_CLASS {
            for c in chars.chars() {
                h.insert(c, (class, val));
            }
        }
        h
    })
}

#[cfg(test)]
mod tests;
