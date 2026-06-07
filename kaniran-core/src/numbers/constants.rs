use super::kani_num_class::NumClass;

/// Port of `ichiran/numbers:*digit-kanji-default*` (`numbers.lisp:3`).
///
/// Default 0–9 kanji glyph table, indexed by digit value.
pub const DIGIT_KANJI_DEFAULT: &str = "〇一二三四五六七八九";

/// Port of `ichiran/numbers:*digit-kanji-legal*` (`numbers.lisp:5`).
///
/// Legal / financial 0–10 kanji glyph table, indexed by digit value.
/// These forms (`壱`, `弐`, `参`, `拾`, ...) are used in contracts and
/// formal contexts where the simpler `一`, `二`, `三`, `十` could be
/// altered.
pub const DIGIT_KANJI_LEGAL: &str = "〇壱弐参四五六七八九拾";

/// Port of `ichiran/numbers:*power-kanji*` (`numbers.lisp:7`).
///
/// Power-of-ten kanji glyph table indexed by exponent. Slot `i` holds
/// the kanji for `10^i`, with ASCII spaces filling the four exponents
/// (`5..=7`, `9..=11`, `13..=15`) where Japanese has no dedicated
/// single-character form.
pub const POWER_KANJI: &str = "一十百千万   億   兆   京";

/// Port of `ichiran/numbers:*char-number-class*` (`numbers.lisp:9`).
///
/// Source-of-truth table mapping each numeric character to its
/// [`NumClass`] tag and value. The first column groups characters that
/// share the same classification (e.g. `"〇零"` are both
/// `(NumClass::Jd, 0)`, `"十拾"` are both `(NumClass::P, 1)`).
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

/// Port of `ichiran/numbers:*digit-to-kana*` (`numbers.lisp:25`).
///
/// Reading for each digit `0..=9` in hiragana. Indexed by digit value;
/// `DIGIT_TO_KANA[n]` is the reading of `n`.
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

/// Port of `ichiran/numbers:*power-to-kana*` (`numbers.lisp:28`).
///
/// Reading for each power-of-ten kanji that has a single-character form,
/// keyed by exponent. Sparse — only `1, 2, 3, 4, 8, 12, 16` appear.
pub const POWER_TO_KANA: &[(u8, &str)] = &[
    (1, "じゅう"),  // 十
    (2, "ひゃく"),  // 百
    (3, "せん"),    // 千
    (4, "まん"),    // 万
    (8, "おく"),    // 億
    (12, "ちょう"), // 兆
    (16, "けい"),   // 京
];
