//! Port of `ichiran/characters:*dakuten-hash*`
//! (`characters.lisp:62-67`).
//!
//! Maps an unvoiced mora `KanaClass` to its voiced counterpart —
//! `Ka → Ga`, `Shi → Ji`, `Ha → Ba`, `U → Vu`, etc. 21 entries.
//! Used by `voice-char` (`characters.lisp:81`) to compute the voiced
//! form of a mora; lookup falls back to the input class itself when
//! absent.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::_star_all_characters_star_::KanaClass;

static CACHE: OnceLock<HashMap<KanaClass, KanaClass>> = OnceLock::new();

pub fn dakuten_hash() -> &'static HashMap<KanaClass, KanaClass> {
    CACHE.get_or_init(|| {
        use KanaClass::*;
        [
            (Ka, Ga),
            (Ki, Gi),
            (Ku, Gu),
            (Ke, Ge),
            (Ko, Go),
            (Sa, Za),
            (Shi, Ji),
            (Su, Zu),
            (Se, Ze),
            (So, Zo),
            (Ta, Da),
            (Chi, Dji),
            (Tsu, Dzu),
            (Te, De),
            (To, Do),
            (Ha, Ba),
            (Hi, Bi),
            (Fu, Bu),
            (He, Be),
            (Ho, Bo),
            (U, Vu),
        ]
        .into_iter()
        .collect()
    })
}
