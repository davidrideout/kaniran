//! Port of `ichiran/characters:voice-char` (`characters.lisp:81-83`).
//!
//! Returns the voiced form of a [`KanaClass`], or the input itself if
//! the class has no voiced counterpart in
//! [`super::_star_dakuten_hash_star_::dakuten_hash`]. Only the dakuten
//! mapping is consulted — handakuten (`Ha → Pa` etc.) is not.

use super::_star_dakuten_hash_star_::dakuten_hash;
use super::kani_kana_class::KanaClass;

pub fn voice_char(cc: KanaClass) -> KanaClass {
    dakuten_hash().get(&cc).copied().unwrap_or(cc)
}
