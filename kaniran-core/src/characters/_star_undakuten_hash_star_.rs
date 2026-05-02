//! Port of `ichiran/characters:*undakuten-hash*`
//! (`characters.lisp:73-79`).
//!
//! Inverse of dakuten + handakuten: maps a voiced or semi-voiced mora
//! `KanaClass` back to its unvoiced base — `Ga → Ka`, `Ji → Shi`,
//! `Ba → Ha`, `Pa → Ha`, `Vu → U`, etc. 26 entries. *This* table is
//! not a perfect inverse of `*dakuten-hash*`: both `Ba` and `Pa`
//! collapse to `Ha`, both `Bi` and `Pi` to `Hi`, and so on — so
//! unvoicing throws away the b/p distinction. Going voiced→unvoiced
//! is lossy in a way that voicing→voiced isn't.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::kani_kana_class::KanaClass;

static CACHE: OnceLock<HashMap<KanaClass, KanaClass>> = OnceLock::new();

pub fn undakuten_hash() -> &'static HashMap<KanaClass, KanaClass> {
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
