//! Port of `ichiran/characters:*handakuten-hash*`
//! (`characters.lisp:70-71`).
//!
//! Maps an unvoiced mora `KanaClass` to its handakuten (semi-voiced /
//! "p") counterpart — `Ha → Pa`, `Hi → Pi`, etc. 5 entries; only the
//! H-row has a handakuten form. Used by `dakuten-join` to generate the
//! precomposed form for a `゜` combining mark, and by `rendaku` when
//! called with `:handakuten t`. (Note: `voice-char` itself only
//! consults `*dakuten-hash*`, not this table.)

use std::collections::HashMap;
use std::sync::OnceLock;

use super::kani_kana_class::KanaClass;

static CACHE: OnceLock<HashMap<KanaClass, KanaClass>> = OnceLock::new();

pub fn handakuten_hash() -> &'static HashMap<KanaClass, KanaClass> {
    CACHE.get_or_init(|| {
        use KanaClass::*;
        [
            (Ha, Pa),
            (Hi, Pi),
            (Fu, Pu),
            (He, Pe),
            (Ho, Po),
        ]
        .into_iter()
        .collect()
    })
}
