//! Port of `ichiran/characters:*undakuten-hash*`
//! (`characters.lisp:73-79`).
//!
//! Inverse of dakuten + handakuten: maps a voiced or semi-voiced mora
//! `KanaClass` back to its unvoiced base — `Ga → Ka`, `Ji → Shi`,
//! `Ba → Ha`, `Pa → Ha`, `Vu → U`, etc. 26 entries. Not a perfect
//! inverse: both `Ba` and `Pa` collapse to `Ha`, so unvoicing throws
//! away the b/p distinction.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::kani_kana_class::KanaClass;

pub fn undakuten_hash() -> &'static HashMap<KanaClass, KanaClass> {
    static CACHE: OnceLock<HashMap<KanaClass, KanaClass>> = OnceLock::new();
    CACHE.get_or_init(|| {
        use KanaClass::*;
        [
            (Ga, Ka),
            (Gi, Ki),
            (Gu, Ku),
            (Ge, Ke),
            (Go, Ko),
            (Za, Sa),
            (Ji, Shi),
            (Zu, Su),
            (Ze, Se),
            (Zo, So),
            (Da, Ta),
            (Dji, Chi),
            (Dzu, Tsu),
            (De, Te),
            (Do, To),
            (Ba, Ha),
            (Bi, Hi),
            (Bu, Fu),
            (Be, He),
            (Bo, Ho),
            (Pa, Ha),
            (Pi, Hi),
            (Pu, Fu),
            (Pe, He),
            (Po, Ho),
            (Vu, U),
        ]
        .into_iter()
        .collect()
    })
}
