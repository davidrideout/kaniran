//! Numeric glyph / reading tables from `numbers.lisp:3-28`.

use super::kani_num_class::NumClass;

/// `*digit-kanji-default*` — everyday 0–9 kanji, indexed by digit.
pub const DIGIT_KANJI_DEFAULT: &str = "〇一二三四五六七八九";

/// `*digit-kanji-legal*` — financial 0–10 kanji (`壱 弐 参 拾`…) used
/// in contracts where the everyday forms could be altered.
pub const DIGIT_KANJI_LEGAL: &str = "〇壱弐参四五六七八九拾";

/// `*power-kanji*` — `10^i` kanji indexed by exponent; ASCII spaces
/// fill exponents `5..=7`, `9..=11`, `13..=15` (no single-char form).
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

/// `*power-to-kana*` — reading per exponent, sparse (only the
/// single-char powers: 1, 2, 3, 4, 8, 12, 16).
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
