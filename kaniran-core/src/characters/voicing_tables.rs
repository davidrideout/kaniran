//! KanaClass voicing maps and the derived `(combining → precomposed)`
//! pair list.
//!
//! Originally three `defparameter` files (`*dakuten-hash*`,
//! `*handakuten-hash*`, `*undakuten-hash*`) plus the derived
//! `*dakuten-join*` under `ichiran/characters`
//! (`characters.lisp:62-104`); consolidated here during phase 2
//! cleanup.
//!
//! [`dakuten_hash`] / [`handakuten_hash`] go unvoiced → (semi-)voiced;
//! [`undakuten_hash`] is the lossy inverse (both `Ba` and `Pa`
//! collapse to `Ha`). [`dakuten_join`] folds dakuten and handakuten
//! into one `(input-with-combining-mark, single-precomposed-char)`
//! table consumed by `simplify-ngrams` to normalize decomposed forms.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::dakuten_join::dakuten_join as build_join;
use super::kani_kana_class::KanaClass;

/// `*dakuten-hash*` — unvoiced mora → voiced counterpart (`Ka → Ga`,
/// `Shi → Ji`, `Ha → Ba`, `U → Vu`, …). 21 entries. Consumed by
/// [`super::voice_char::voice_char`].
pub fn dakuten_hash() -> &'static HashMap<KanaClass, KanaClass> {
    static CACHE: OnceLock<HashMap<KanaClass, KanaClass>> = OnceLock::new();
    CACHE.get_or_init(|| {
        use KanaClass::*;
        [
            (Ka, Ga), (Ki, Gi), (Ku, Gu), (Ke, Ge), (Ko, Go),
            (Sa, Za), (Shi, Ji), (Su, Zu), (Se, Ze), (So, Zo),
            (Ta, Da), (Chi, Dji), (Tsu, Dzu), (Te, De), (To, Do),
            (Ha, Ba), (Hi, Bi), (Fu, Bu), (He, Be), (Ho, Bo),
            (U, Vu),
        ]
        .into_iter()
        .collect()
    })
}

/// `*handakuten-hash*` — unvoiced mora → handakuten (semi-voiced "p")
/// counterpart (`Ha → Pa`, `Hi → Pi`, …). 5 entries; only the H-row
/// has a handakuten form. Used by `dakuten-join` and by `rendaku`
/// with `:handakuten t`.
pub fn handakuten_hash() -> &'static HashMap<KanaClass, KanaClass> {
    static CACHE: OnceLock<HashMap<KanaClass, KanaClass>> = OnceLock::new();
    CACHE.get_or_init(|| {
        use KanaClass::*;
        [(Ha, Pa), (Hi, Pi), (Fu, Pu), (He, Pe), (Ho, Po)]
            .into_iter()
            .collect()
    })
}

/// `*undakuten-hash*` — voiced or semi-voiced mora → unvoiced base
/// (`Ga → Ka`, `Ji → Shi`, `Ba → Ha`, `Pa → Ha`, `Vu → U`, …). 26
/// entries. Not a perfect inverse of [`dakuten_hash`]: both `Ba` and
/// `Pa` collapse to `Ha`, both `Bi` and `Pi` to `Hi`, etc. — so
/// unvoicing throws away the b/p distinction.
pub fn undakuten_hash() -> &'static HashMap<KanaClass, KanaClass> {
    static CACHE: OnceLock<HashMap<KanaClass, KanaClass>> = OnceLock::new();
    CACHE.get_or_init(|| {
        use KanaClass::*;
        [
            (Ga, Ka), (Gi, Ki), (Gu, Ku), (Ge, Ke), (Go, Ko),
            (Za, Sa), (Ji, Shi), (Zu, Su), (Ze, Se), (Zo, So),
            (Da, Ta), (Dji, Chi), (Dzu, Tsu), (De, Te), (Do, To),
            (Ba, Ha), (Bi, Hi), (Bu, Fu), (Be, He), (Bo, Ho),
            (Pa, Ha), (Pi, Hi), (Pu, Fu), (Pe, He), (Po, Ho),
            (Vu, U),
        ]
        .into_iter()
        .collect()
    })
}

/// `*dakuten-join*` — pairs of `(input-with-combining-mark,
/// single-precomposed-char)` for both dakuten (`゛`) and handakuten
/// (`゜`). Derived at first use by calling
/// [`super::dakuten_join::dakuten_join`] on the dakuten and
/// handakuten hashes and concatenating the results — exact upstream
/// construction.
pub fn dakuten_join() -> &'static Vec<(String, String)> {
    static CACHE: OnceLock<Vec<(String, String)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut v = build_join(dakuten_hash(), '゛');
        v.extend(build_join(handakuten_hash(), '゜'));
        v
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured by the introspector running
    /// `(append (dakuten-join *dakuten-hash* #\゛) (dakuten-join *handakuten-hash* #\゜))`
    /// against the upstream image. SBCL's `hash-table-alist` order is
    /// implementation-defined, so the upstream pair order itself is
    /// not stable — the comparison sorts both sides.
    static INTROSPECTED: &[(&str, &str)] = &[
        ("う゛", "ゔ"), ("ウ゛", "ヴ"),
        ("ほ゛", "ぼ"), ("ホ゛", "ボ"),
        ("へ゛", "べ"), ("ヘ゛", "ベ"),
        ("ふ゛", "ぶ"), ("フ゛", "ブ"),
        ("ひ゛", "び"), ("ヒ゛", "ビ"),
        ("は゛", "ば"), ("ハ゛", "バ"),
        ("と゛", "ど"), ("ト゛", "ド"),
        ("て゛", "で"), ("テ゛", "デ"),
        ("つ゛", "づ"), ("ツ゛", "ヅ"),
        ("ち゛", "ぢ"), ("チ゛", "ヂ"),
        ("た゛", "だ"), ("タ゛", "ダ"),
        ("そ゛", "ぞ"), ("ソ゛", "ゾ"),
        ("せ゛", "ぜ"), ("セ゛", "ゼ"),
        ("す゛", "ず"), ("ス゛", "ズ"),
        ("し゛", "じ"), ("シ゛", "ジ"),
        ("さ゛", "ざ"), ("サ゛", "ザ"),
        ("こ゛", "ご"), ("コ゛", "ゴ"),
        ("け゛", "げ"), ("ケ゛", "ゲ"),
        ("く゛", "ぐ"), ("ク゛", "グ"),
        ("き゛", "ぎ"), ("キ゛", "ギ"),
        ("か゛", "が"), ("カ゛", "ガ"),
        ("ほ゜", "ぽ"), ("ホ゜", "ポ"),
        ("へ゜", "ぺ"), ("ヘ゜", "ペ"),
        ("ふ゜", "ぷ"), ("フ゜", "プ"),
        ("ひ゜", "ぴ"), ("ヒ゜", "ピ"),
        ("は゜", "ぱ"), ("ハ゜", "パ"),
    ];

    #[test]
    fn derived_value_matches_introspected_literal_under_sort() {
        let mut derived: Vec<(&str, &str)> = dakuten_join()
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        derived.sort();
        let mut expected: Vec<(&str, &str)> = INTROSPECTED.to_vec();
        expected.sort();
        assert_eq!(derived, expected);
    }
}
