//! Port of `ichiran/characters:*char-class-hash*`
//! (`characters.lisp:37`).
//!
//! Reverse map from individual kana glyphs to their [`KanaClass`]
//! (e.g. `'っ' → KanaClass::Sokuon`, `'ア' → KanaClass::A`). Built once
//! on first access by walking `*all-characters*` and exploding each
//! `chars` string into per-character entries — exactly mirroring the
//! Lisp `loop ... do (setf (gethash char hash) class)` construction.
//!
//! Lookups for a non-kana char return `None`; in the Lisp the lookup
//! falls back to the input char itself, which made the result a
//! heterogeneous "tag-or-char" value. Rust callers reconstruct that
//! shape at the lookup site (e.g. wrap in an `Either<KanaClass, char>`)
//! when the consuming function is ported.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::_star_all_characters_star_::{ALL_CHARACTERS, KanaClass};

static CACHE: OnceLock<HashMap<char, KanaClass>> = OnceLock::new();

pub fn char_class_hash() -> &'static HashMap<char, KanaClass> {
    CACHE.get_or_init(|| {
        let mut h = HashMap::new();
        for (class, chars) in ALL_CHARACTERS {
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
