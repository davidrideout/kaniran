//! Port of `ichiran/dict:*hint-char-map*` (`dict-split.lisp:816`).
//!
//! Plist mapping each [`super::kani_hint_kind::KaniHintKind`] tag to
//! the sentinel character the hint system splices into a kana string
//! at that tag's position. Looked up by [`super::insert_hints`]
//! (which mirrors `(getf *hint-char-map* character-kw)`) and scanned
//! by [`super::strip_hints`] (which removes any char appearing as a
//! value here).
//!
//! ## Divergence
//!
//! The Lisp `defparameter` is a flat plist
//! `(:space ,*kana-hint-space* :mod ,*kana-hint-mod*)`. The Rust port
//! holds the same key/value pairs as a typed slice of
//! `(KaniHintKind, char)` rather than a heterogeneous plist; both
//! consumers (`insert-hints`, `strip-hints`) only ever scan it
//! pairwise.

use super::_star_kana_hint_mod_star_::KANA_HINT_MOD;
use super::_star_kana_hint_space_star_::KANA_HINT_SPACE;
use super::kani_hint_kind::KaniHintKind;

pub const HINT_CHAR_MAP: [(KaniHintKind, char); 2] = [
    (KaniHintKind::Space, KANA_HINT_SPACE),
    (KaniHintKind::Mod, KANA_HINT_MOD),
];
