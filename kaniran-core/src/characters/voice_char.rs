//! Port of `ichiran/characters:voice-char` (`characters.lisp:81-83`).
//!
//! Returns the voiced form of a [`KanaClass`], or the input itself if
//! the class has no voiced counterpart in
//! [`super::voicing_tables::dakuten_hash`]. Only the dakuten
//! mapping is consulted — handakuten (`Ha → Pa` etc.) is not.
//!
//! The Lisp idiom `(gethash cc *dakuten-hash* cc)` falls back to the
//! key when missing. Per CONVENTIONS §4.2 the same-typed default
//! collapses directly to `unwrap_or` — no `Option` shape needed,
//! because input and output are both `KanaClass`.

use super::voicing_tables::dakuten_hash;
use super::kani_kana_class::KanaClass;

pub fn voice_char(cc: KanaClass) -> KanaClass {
    dakuten_hash().get(&cc).copied().unwrap_or(cc)
}
