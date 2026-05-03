//! Port of `ichiran/numbers:*char-number-class-hash*` (`numbers.lisp:18`).
//!
//! Per-character lookup from any numeric glyph to its `(NumClass, u8)`
//! classification. Built lazily on first access by exploding every
//! group in [`super::_star_char_number_class_star_::CHAR_NUMBER_CLASS`]
//! — same shape and order as the upstream `loop ... do ... do` builder.
//! Used by [`super::parse_number::parse_number`] and
//! [`super::number_to_kana::number_to_kana`] to classify a single glyph.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::_star_char_number_class_star_::CHAR_NUMBER_CLASS;
use super::kani_num_class::NumClass;

static CACHE: OnceLock<HashMap<char, (NumClass, u8)>> = OnceLock::new();

pub fn char_number_class_hash() -> &'static HashMap<char, (NumClass, u8)> {
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
mod tests {
    use super::*;

    /// Guards the explode-from-`CHAR_NUMBER_CLASS` build loop. 42 is
    /// what the introspector captured from the live Lisp hashtable;
    /// mismatches flag a regression in the loop, not a typo in the
    /// source table.
    #[test]
    fn build_logic_produces_42_entries() {
        assert_eq!(char_number_class_hash().len(), 42);
    }
}
