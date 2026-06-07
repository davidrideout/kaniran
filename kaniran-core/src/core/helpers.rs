use super::deromanize::{has_successors, RmapItem};
use crate::characters::kani_kana_class::KanaClass;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

/// Port of `ichiran:*hepburn-kana-table*` (`romanize.lisp:81`).
pub fn hepburn_kana_table() -> &'static HashMap<KanaClass, &'static str> {
    static CACHE: OnceLock<HashMap<KanaClass, &'static str>> = OnceLock::new();
    CACHE.get_or_init(|| {
        use KanaClass::*;
        [
            (A, "a"),
            (I, "i"),
            (U, "u"),
            (E, "e"),
            (O, "o"),
            (Ka, "ka"),
            (Ki, "ki"),
            (Ku, "ku"),
            (Ke, "ke"),
            (Ko, "ko"),
            (Sa, "sa"),
            (Shi, "shi"),
            (Su, "su"),
            (Se, "se"),
            (So, "so"),
            (Ta, "ta"),
            (Chi, "chi"),
            (Tsu, "tsu"),
            (Te, "te"),
            (To, "to"),
            (Na, "na"),
            (Ni, "ni"),
            (Nu, "nu"),
            (Ne, "ne"),
            (No, "no"),
            (Ha, "ha"),
            (Hi, "hi"),
            (Fu, "fu"),
            (He, "he"),
            (Ho, "ho"),
            (Ma, "ma"),
            (Mi, "mi"),
            (Mu, "mu"),
            (Me, "me"),
            (Mo, "mo"),
            (Ya, "ya"),
            (Yu, "yu"),
            (Yo, "yo"),
            (Ra, "ra"),
            (Ri, "ri"),
            (Ru, "ru"),
            (Re, "re"),
            (Ro, "ro"),
            (Wa, "wa"),
            (Wi, "wi"),
            (We, "we"),
            (Wo, "wo"),
            (N, "n'"),
            (Ga, "ga"),
            (Gi, "gi"),
            (Gu, "gu"),
            (Ge, "ge"),
            (Go, "go"),
            (Za, "za"),
            (Ji, "ji"),
            (Zu, "zu"),
            (Ze, "ze"),
            (Zo, "zo"),
            (Da, "da"),
            (Dji, "ji"),
            (Dzu, "zu"),
            (De, "de"),
            (Do, "do"),
            (Ba, "ba"),
            (Bi, "bi"),
            (Bu, "bu"),
            (Be, "be"),
            (Bo, "bo"),
            (Pa, "pa"),
            (Pi, "pi"),
            (Pu, "pu"),
            (Pe, "pe"),
            (Po, "po"),
            (PlusA, "a"),
            (PlusI, "i"),
            (PlusU, "u"),
            (PlusE, "e"),
            (PlusO, "o"),
            (PlusYa, "ya"),
            (PlusYu, "yu"),
            (PlusYo, "yo"),
            (Vu, "vu"),
            (PlusWa, "wa"),
        ]
        .into_iter()
        .collect()
    })
}

/// Port of `ichiran:*kunrei-siki-kana-table*` (`romanize.lisp:172`).
pub fn kunrei_siki_kana_table() -> &'static HashMap<KanaClass, &'static str> {
    static CACHE: OnceLock<HashMap<KanaClass, &'static str>> = OnceLock::new();
    CACHE.get_or_init(|| {
        use KanaClass::*;
        [
            (A, "a"),
            (I, "i"),
            (U, "u"),
            (E, "e"),
            (O, "o"),
            (Ka, "ka"),
            (Ki, "ki"),
            (Ku, "ku"),
            (Ke, "ke"),
            (Ko, "ko"),
            (Sa, "sa"),
            (Shi, "si"),
            (Su, "su"),
            (Se, "se"),
            (So, "so"),
            (Ta, "ta"),
            (Chi, "ti"),
            (Tsu, "tu"),
            (Te, "te"),
            (To, "to"),
            (Na, "na"),
            (Ni, "ni"),
            (Nu, "nu"),
            (Ne, "ne"),
            (No, "no"),
            (Ha, "ha"),
            (Hi, "hi"),
            (Fu, "hu"),
            (He, "he"),
            (Ho, "ho"),
            (Ma, "ma"),
            (Mi, "mi"),
            (Mu, "mu"),
            (Me, "me"),
            (Mo, "mo"),
            (Ya, "ya"),
            (Yu, "yu"),
            (Yo, "yo"),
            (Ra, "ra"),
            (Ri, "ri"),
            (Ru, "ru"),
            (Re, "re"),
            (Ro, "ro"),
            (Wa, "wa"),
            (Wi, "i"),
            (We, "e"),
            (Wo, "o"),
            (N, "n'"),
            (Ga, "ga"),
            (Gi, "gi"),
            (Gu, "gu"),
            (Ge, "ge"),
            (Go, "go"),
            (Za, "za"),
            (Ji, "zi"),
            (Zu, "zu"),
            (Ze, "ze"),
            (Zo, "zo"),
            (Da, "da"),
            (Dji, "zi"),
            (Dzu, "zu"),
            (De, "de"),
            (Do, "do"),
            (Ba, "ba"),
            (Bi, "bi"),
            (Bu, "bu"),
            (Be, "be"),
            (Bo, "bo"),
            (Pa, "pa"),
            (Pi, "pi"),
            (Pu, "pu"),
            (Pe, "pe"),
            (Po, "po"),
            (PlusA, "a"),
            (PlusI, "i"),
            (PlusU, "u"),
            (PlusE, "e"),
            (PlusO, "o"),
            (PlusYa, "ya"),
            (PlusYu, "yu"),
            (PlusYo, "yo"),
            (Vu, "vu"),
            (PlusWa, "wa"),
        ]
        .into_iter()
        .collect()
    })
}

/// Port of `ichiran:*romaji-kana*` (`deromanize.lisp:7`, `csv-hash *romaji-kana*`).
///
/// Maps a romaji prefix to its [`RmapItem`] kana rule.
pub fn romaji_kana() -> &'static HashMap<String, RmapItem> {
    static ROMAJI_KANA: OnceLock<HashMap<String, RmapItem>> = OnceLock::new();
    ROMAJI_KANA.get_or_init(load_romaji_kana)
}

/// Port of `ichiran:get-romaji-kana` (`deromanize.lisp:7`, `csv-hash *romaji-kana*` expansion).
///
/// Looks up the romaji prefix `key` in the romaji-map, returning its
/// [`RmapItem`] rule or `None` when absent.
pub fn get_romaji_kana(key: &str) -> Option<&'static RmapItem> {
    romaji_kana().get(key)
}

/// Port of `ichiran:load-romaji-kana` (`deromanize.lisp:7`, `csv-hash *romaji-kana*` expansion).
///
/// Builds the romaji-prefix → [`RmapItem`] map from romaji-map.csv
/// (tab-separated `text<TAB>kana` rows, optional third `next` column on
/// doubled-consonant gemination rows). The `text` column is the key, so
/// the duplicate `fu` row collapses to one entry (292 keys from 293 rows).
const ROMAJI_MAP_CSV: &str = include_str!("../../data/romaji-map.csv");

pub fn load_romaji_kana() -> HashMap<String, RmapItem> {
    let mut hash = HashMap::new();
    for row in ROMAJI_MAP_CSV.lines() {
        let mut cols = row.split('\t');
        let text = cols.next().expect("romaji-map.csv row missing text column");
        let kana = cols.next().expect("romaji-map.csv row missing kana column");
        let next = cols.next();
        hash.insert(
            text.to_string(),
            RmapItem {
                text: text.to_string(),
                kana: kana.to_string(),
                next: next.map(str::to_string),
            },
        );
    }
    hash
}

/// Port of `ichiran:*romaji-kana-next*` (`deromanize.lisp:21`).
///
/// Set of every proper prefix of every romaji key in
/// [`*romaji-kana*`][super::helpers::romaji_kana] —
/// the "could this grow into a longer key?" membership test consulted
/// by `romaji-next`.
pub fn romaji_kana_next() -> &'static HashSet<String> {
    static ROMAJI_KANA_NEXT: OnceLock<HashSet<String>> = OnceLock::new();
    ROMAJI_KANA_NEXT.get_or_init(|| {
        let keys: Vec<&str> = romaji_kana().keys().map(String::as_str).collect();
        has_successors(&keys)
    })
}

#[cfg(test)]
mod tests;
