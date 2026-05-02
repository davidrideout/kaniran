//! Port of `ichiran/characters:*char-class-hash*`
//! (`characters.lisp:37`).
//!
//! Reverse map from individual kana glyphs to their [`KanaClass`]
//! (e.g. `'っ' → KanaClass::Sokuon`, `'ア' → KanaClass::A`). Built once
//! on first access by walking `*all-characters*` and exploding each
//! `chars` string into per-character entries — exactly mirroring the
//! upstream construction.
//!
//! Lookups on a non-kana char return `None`; the Lisp hashtable
//! returns `nil` in the same case. The "tag-or-char fallback" lives
//! one level up in `get-char-class` (which substitutes the input char
//! itself when the table misses), not in this table.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::_star_all_characters_star_::all_characters;
use super::kani_kana_class::KanaClass;

static CACHE: OnceLock<HashMap<char, KanaClass>> = OnceLock::new();

pub fn char_class_hash() -> &'static HashMap<char, KanaClass> {
    CACHE.get_or_init(|| {
        let mut h = HashMap::new();
        for (class, chars) in all_characters() {
            for c in chars.chars() {
                h.insert(c, *class);
            }
        }
        h
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Guards the derive-from-`ALL_CHARACTERS` build logic. 173 entries
    /// is what the Lisp introspection captured; mismatches here flag a
    /// regression in the loop, not a typo in the data.
    #[test]
    fn build_logic_produces_173_entries() {
        assert_eq!(char_class_hash().len(), 173);
    }
}
