//! Per-method kana romanization tables. From `romanize.lisp:81-100`
//! and `:172-191`.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::characters::kana_class::KanaClass;

/// `*hepburn-kana-table*` (`romanize.lisp:81`).
pub fn hepburn_kana_table() -> &'static HashMap<KanaClass, &'static str> {
    static CACHE: OnceLock<HashMap<KanaClass, &'static str>> = OnceLock::new();
    CACHE.get_or_init(|| {
        use KanaClass::*;
        [
            (A, "a"), (I, "i"), (U, "u"), (E, "e"), (O, "o"),
            (Ka, "ka"), (Ki, "ki"), (Ku, "ku"), (Ke, "ke"), (Ko, "ko"),
            (Sa, "sa"), (Shi, "shi"), (Su, "su"), (Se, "se"), (So, "so"),
            (Ta, "ta"), (Chi, "chi"), (Tsu, "tsu"), (Te, "te"), (To, "to"),
            (Na, "na"), (Ni, "ni"), (Nu, "nu"), (Ne, "ne"), (No, "no"),
            (Ha, "ha"), (Hi, "hi"), (Fu, "fu"), (He, "he"), (Ho, "ho"),
            (Ma, "ma"), (Mi, "mi"), (Mu, "mu"), (Me, "me"), (Mo, "mo"),
            (Ya, "ya"), (Yu, "yu"), (Yo, "yo"),
            (Ra, "ra"), (Ri, "ri"), (Ru, "ru"), (Re, "re"), (Ro, "ro"),
            (Wa, "wa"), (Wi, "wi"), (We, "we"), (Wo, "wo"),
            (N, "n'"),
            (Ga, "ga"), (Gi, "gi"), (Gu, "gu"), (Ge, "ge"), (Go, "go"),
            (Za, "za"), (Ji, "ji"), (Zu, "zu"), (Ze, "ze"), (Zo, "zo"),
            (Da, "da"), (Dji, "ji"), (Dzu, "zu"), (De, "de"), (Do, "do"),
            (Ba, "ba"), (Bi, "bi"), (Bu, "bu"), (Be, "be"), (Bo, "bo"),
            (Pa, "pa"), (Pi, "pi"), (Pu, "pu"), (Pe, "pe"), (Po, "po"),
            (PlusA, "a"), (PlusI, "i"), (PlusU, "u"), (PlusE, "e"), (PlusO, "o"),
            (PlusYa, "ya"), (PlusYu, "yu"), (PlusYo, "yo"),
            (Vu, "vu"), (PlusWa, "wa"),
        ]
        .into_iter()
        .collect()
    })
}

/// `*kunrei-siki-kana-table*` (`romanize.lisp:172`).
pub fn kunrei_siki_kana_table() -> &'static HashMap<KanaClass, &'static str> {
    static CACHE: OnceLock<HashMap<KanaClass, &'static str>> = OnceLock::new();
    CACHE.get_or_init(|| {
        use KanaClass::*;
        [
            (A, "a"), (I, "i"), (U, "u"), (E, "e"), (O, "o"),
            (Ka, "ka"), (Ki, "ki"), (Ku, "ku"), (Ke, "ke"), (Ko, "ko"),
            (Sa, "sa"), (Shi, "si"), (Su, "su"), (Se, "se"), (So, "so"),
            (Ta, "ta"), (Chi, "ti"), (Tsu, "tu"), (Te, "te"), (To, "to"),
            (Na, "na"), (Ni, "ni"), (Nu, "nu"), (Ne, "ne"), (No, "no"),
            (Ha, "ha"), (Hi, "hi"), (Fu, "hu"), (He, "he"), (Ho, "ho"),
            (Ma, "ma"), (Mi, "mi"), (Mu, "mu"), (Me, "me"), (Mo, "mo"),
            (Ya, "ya"), (Yu, "yu"), (Yo, "yo"),
            (Ra, "ra"), (Ri, "ri"), (Ru, "ru"), (Re, "re"), (Ro, "ro"),
            (Wa, "wa"), (Wi, "i"), (We, "e"), (Wo, "o"),
            (N, "n'"),
            (Ga, "ga"), (Gi, "gi"), (Gu, "gu"), (Ge, "ge"), (Go, "go"),
            (Za, "za"), (Ji, "zi"), (Zu, "zu"), (Ze, "ze"), (Zo, "zo"),
            (Da, "da"), (Dji, "zi"), (Dzu, "zu"), (De, "de"), (Do, "do"),
            (Ba, "ba"), (Bi, "bi"), (Bu, "bu"), (Be, "be"), (Bo, "bo"),
            (Pa, "pa"), (Pi, "pi"), (Pu, "pu"), (Pe, "pe"), (Po, "po"),
            (PlusA, "a"), (PlusI, "i"), (PlusU, "u"), (PlusE, "e"), (PlusO, "o"),
            (PlusYa, "ya"), (PlusYu, "yu"), (PlusYo, "yo"),
            (Vu, "vu"), (PlusWa, "wa"),
        ]
        .into_iter()
        .collect()
    })
}
