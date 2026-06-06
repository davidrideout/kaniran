//! Port of `ichiran/characters:*handakuten-hash*`
//! (`characters.lisp:70-71`).
//!
//! Maps an unvoiced mora `KanaClass` to its handakuten (semi-voiced /
//! "p") counterpart — `Ha → Pa`, `Hi → Pi`, etc. Only the H-row has a
//! handakuten form.

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
