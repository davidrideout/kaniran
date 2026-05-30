//! Numeric-class tag enum, glyph tables, and per-character lookup.
//! From `numbers.lisp:3-28`.
//!
//! Lisp uses inline `:jd` / `:p` / `:ad` keywords without a named type;
//! [`NumClass`] is the closed Rust enumeration of that implicit set.

use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumClass {
    /// Japanese digit kanji (`〇一二三四五六七八九` + legal forms),
    /// value `0..=9`.
    Jd,
    /// Power-of-ten kanji (`十百千万億兆京`), value is the exponent
    /// (`1, 2, 3, 4, 8, 12, 16`).
    P,
    /// ASCII / full-width digit (`0-9`, `０-９`), value `0..=9`.
    Ad,
}

/// `*digit-kanji-default*` — everyday 0–9 kanji, indexed by digit.
pub const DIGIT_KANJI_DEFAULT: &str = "〇一二三四五六七八九";

/// `*digit-kanji-legal*` — financial 0–10 kanji (`壱 弐 参 拾`…).
pub const DIGIT_KANJI_LEGAL: &str = "〇壱弐参四五六七八九拾";

/// `*power-kanji*` — `10^i` indexed by exponent; ASCII spaces fill
/// exponents `5..=7`, `9..=11`, `13..=15` (no single-char form).
pub const POWER_KANJI: &str = "一十百千万   億   兆   京";

/// `*digit-to-kana*` — hiragana reading for digits 0..=9.
pub const DIGIT_TO_KANA: &[&str] = &[
    "れい",   // 0
    "いち",   // 1
    "に",     // 2
    "さん",   // 3
    "よん",   // 4
    "ご",     // 5
    "ろく",   // 6
    "なな",   // 7
    "はち",   // 8
    "きゅう", // 9
];

/// `*power-to-kana*` — reading per exponent, sparse (1, 2, 3, 4, 8, 12, 16).
pub const POWER_TO_KANA: &[(u8, &str)] = &[
    (1, "じゅう"),  // 十
    (2, "ひゃく"),  // 百
    (3, "せん"),    // 千
    (4, "まん"),    // 万
    (8, "おく"),    // 億
    (12, "ちょう"), // 兆
    (16, "けい"),   // 京
];

/// `*char-number-class*` — numeric glyph → `(NumClass, value)`. First
/// column groups glyphs that share a classification (`"〇零"` are both
/// `(Jd, 0)`).
pub const CHAR_NUMBER_CLASS: &[(&str, NumClass, u8)] = &[
    ("〇零", NumClass::Jd, 0),
    ("一壱", NumClass::Jd, 1),
    ("二弐", NumClass::Jd, 2),
    ("三参", NumClass::Jd, 3),
    ("四", NumClass::Jd, 4),
    ("五", NumClass::Jd, 5),
    ("六", NumClass::Jd, 6),
    ("七", NumClass::Jd, 7),
    ("八", NumClass::Jd, 8),
    ("九", NumClass::Jd, 9),
    ("十拾", NumClass::P, 1),
    ("百", NumClass::P, 2),
    ("千", NumClass::P, 3),
    ("万", NumClass::P, 4),
    ("億", NumClass::P, 8),
    ("兆", NumClass::P, 12),
    ("京", NumClass::P, 16),
    ("0０", NumClass::Ad, 0),
    ("1１", NumClass::Ad, 1),
    ("2２", NumClass::Ad, 2),
    ("3３", NumClass::Ad, 3),
    ("4４", NumClass::Ad, 4),
    ("5５", NumClass::Ad, 5),
    ("6６", NumClass::Ad, 6),
    ("7７", NumClass::Ad, 7),
    ("8８", NumClass::Ad, 8),
    ("9９", NumClass::Ad, 9),
];

/// `*char-number-class-hash*` (`numbers.lisp:18`). Per-character lookup
/// built by exploding every group in [`CHAR_NUMBER_CLASS`].
pub fn char_number_class_hash() -> &'static HashMap<char, (NumClass, u8)> {
    static CACHE: OnceLock<HashMap<char, (NumClass, u8)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut h = HashMap::new();
        for &(chars, class, val) in CHAR_NUMBER_CLASS {
            for c in chars.chars() {
                h.insert(c, (class, val));
            }
        }
        h
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 42 entries per the Lisp introspector.
    #[test]
    fn build_logic_produces_42_entries() {
        assert_eq!(char_number_class_hash().len(), 42);
    }
}
