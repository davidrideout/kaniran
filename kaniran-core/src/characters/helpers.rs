use super::char_class::CharClass;
use super::constants::{
    CHAR_CLASS_REGEX_MAPPING, DECIMAL_POINT_REGEX, DIGIT_REGEX, FULL_WIDTH_KANA,
    ITERATION_CHARACTERS, KANA_CHARACTERS, MODIFIER_CHARACTERS, NUM_WORD_REGEX, SOKUON_CHARACTERS,
    WORD_REGEX,
};
use super::kani_kana_class::KanaClass;
use super::voicing::dakuten_join as build;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Port of `ichiran/characters:*dakuten-hash*`
/// (`characters.lisp:62-67`).
///
/// Maps an unvoiced mora `KanaClass` to its voiced counterpart —
/// `Ka → Ga`, `Shi → Ji`, `Ha → Ba`, `U → Vu`, etc. 21 entries.
pub fn dakuten_hash() -> &'static HashMap<KanaClass, KanaClass> {
    static CACHE: OnceLock<HashMap<KanaClass, KanaClass>> = OnceLock::new();
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

/// Port of `ichiran/characters:*handakuten-hash*`
/// (`characters.lisp:70-71`).
///
/// Maps an unvoiced mora `KanaClass` to its handakuten (semi-voiced /
/// "p") counterpart — `Ha → Pa`, `Hi → Pi`, etc. Only the H-row has a
/// handakuten form.
pub fn handakuten_hash() -> &'static HashMap<KanaClass, KanaClass> {
    static CACHE: OnceLock<HashMap<KanaClass, KanaClass>> = OnceLock::new();
    CACHE.get_or_init(|| {
        use KanaClass::*;
        [(Ha, Pa), (Hi, Pi), (Fu, Pu), (He, Pe), (Ho, Po)]
            .into_iter()
            .collect()
    })
}

/// Port of `ichiran/characters:*undakuten-hash*`
/// (`characters.lisp:73-79`).
///
/// Inverse of dakuten + handakuten: maps a voiced or semi-voiced mora
/// `KanaClass` back to its unvoiced base — `Ga → Ka`, `Ji → Shi`,
/// `Ba → Ha`, `Pa → Ha`, `Vu → U`, etc. 26 entries. Not a perfect
/// inverse: both `Ba` and `Pa` collapse to `Ha`, so unvoicing throws
/// away the b/p distinction.
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

/// Port of `ichiran/characters:*all-characters*`
/// (`characters.lisp:32-35`).
///
/// Flat list of `(class, chars)` pairs covering every kana mora /
/// modifier / iteration mark recognized by ichiran. Each `chars` string
/// holds the hiragana form followed by the katakana form, except
/// [`KanaClass::LongVowel`] which has the single character `ー`.
pub fn all_characters() -> &'static [(KanaClass, &'static str)] {
    static CACHE: OnceLock<Vec<(KanaClass, &'static str)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut v = Vec::with_capacity(
            SOKUON_CHARACTERS.len()
                + ITERATION_CHARACTERS.len()
                + MODIFIER_CHARACTERS.len()
                + KANA_CHARACTERS.len(),
        );
        v.extend_from_slice(SOKUON_CHARACTERS);
        v.extend_from_slice(ITERATION_CHARACTERS);
        v.extend_from_slice(MODIFIER_CHARACTERS);
        v.extend_from_slice(KANA_CHARACTERS);
        v
    })
}

/// Port of `ichiran/characters:*char-class-hash*`
/// (`characters.lisp:37`).
///
/// Reverse map from individual kana glyphs to their [`KanaClass`]
/// (e.g. `'っ' → KanaClass::Sokuon`, `'ア' → KanaClass::A`).
pub fn char_class_hash() -> &'static HashMap<char, KanaClass> {
    static CACHE: OnceLock<HashMap<char, KanaClass>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut h = HashMap::new();
        for (class, chars) in all_characters() {
            for c in chars.chars() {
                h.insert(c, *class);
            }
        }
        h
    })
}

/// Port of `ichiran/characters:*dakuten-join*` (`characters.lisp:103-104`).
///
/// Pairs of `(input-with-combining-mark, single-precomposed-char)` for
/// both dakuten (゛) and handakuten (゜), normalizing decomposed forms
/// (`"か゛"` → `"が"`) into single code points.
pub fn dakuten_join() -> &'static Vec<(String, String)> {
    static CACHE: OnceLock<Vec<(String, String)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut v = build(dakuten_hash(), '゛');
        v.extend(build(handakuten_hash(), '゜'));
        v
    })
}

/// Port of `ichiran/characters:*normal-chars*`
/// (`characters.lisp:114-116`).
///
/// Standard ASCII printables followed by full-width katakana — the
/// target side of the abnormal→normal character map.
const ASCII_PREFIX: &str =
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ#$%&()*+/<=>?@[]^_`{|}~";

pub fn normal_chars() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| format!("{ASCII_PREFIX}{FULL_WIDTH_KANA}"))
}

/// Port of `ichiran/characters:*basic-split-regex*`
/// (`characters.lisp:131`).
///
/// Composite pattern that recognizes one "word" token in
/// Japanese-mixed-with-digits text.
pub fn basic_split_regex() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| {
        format!(
            "((?:(?<!{}|{}){}+|{}){}*{}|{})",
            DECIMAL_POINT_REGEX,
            DIGIT_REGEX,
            DIGIT_REGEX,
            WORD_REGEX,
            NUM_WORD_REGEX,
            WORD_REGEX,
            WORD_REGEX,
        )
    })
}

/// Port of `ichiran/characters:*char-scanners*`
/// (`characters.lisp:151`).
///
/// Per-`CharClass` compiled regex scanner anchored to the full string:
/// each pattern is wrapped as `^pattern+$`.
pub fn char_scanners() -> &'static HashMap<CharClass, fancy_regex::Regex> {
    static CACHE: OnceLock<HashMap<CharClass, fancy_regex::Regex>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut h = HashMap::new();
        for (class, pat) in CHAR_CLASS_REGEX_MAPPING {
            let wrapped = format!("^{pat}+$");
            let re = fancy_regex::Regex::new(&wrapped)
                .unwrap_or_else(|e| panic!("class {class:?} pattern {wrapped:?} failed: {e}"));
            h.insert(*class, re);
        }
        h
    })
}

/// Port of `ichiran/characters:*char-scanners-inner*`
/// (`characters.lisp:155`).
///
/// Like [`super::_star_char_scanners_star_`] but unanchored: each
/// pattern is wrapped as `(?:pattern)+`, matching one or more
/// characters of the class.
pub fn char_scanners_inner() -> &'static HashMap<CharClass, fancy_regex::Regex> {
    static CACHE: OnceLock<HashMap<CharClass, fancy_regex::Regex>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut h = HashMap::new();
        for (class, pat) in CHAR_CLASS_REGEX_MAPPING {
            let wrapped = format!("(?:{pat})+");
            let re = fancy_regex::Regex::new(&wrapped)
                .unwrap_or_else(|e| panic!("class {class:?} pattern {wrapped:?} failed: {e}"));
            h.insert(*class, re);
        }
        h
    })
}

#[cfg(test)]
mod tests;
