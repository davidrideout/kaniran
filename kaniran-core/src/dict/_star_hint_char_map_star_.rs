//! Port of `ichiran/dict:*hint-char-map*` (`dict-split.lisp:816`).
//!
//! Maps each [`super::kani_hint_kind::KaniHintKind`] tag to the
//! sentinel character the hint system splices into a kana string at
//! that tag's position.

use super::_star_kana_hint_mod_star_::KANA_HINT_MOD;
use super::_star_kana_hint_space_star_::KANA_HINT_SPACE;
use super::kani_hint_kind::KaniHintKind;

pub const HINT_CHAR_MAP: [(KaniHintKind, char); 2] = [
    (KaniHintKind::Space, KANA_HINT_SPACE),
    (KaniHintKind::Mod, KANA_HINT_MOD),
];
