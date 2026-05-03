//! Port of `ichiran/numbers:*power-to-kana*` (`numbers.lisp:28`).
//!
//! Reading for each power-of-ten kanji that has a single-character form,
//! keyed by exponent. Sparse — only `1, 2, 3, 4, 8, 12, 16` appear.
//! Used by [`super::group_to_kana::group_to_kana`] to emit the kana for
//! the [`super::kani_num_class::NumClass::P`] half of a number group.

pub const POWER_TO_KANA: &[(u8, &str)] = &[
    (1, "じゅう"),  // 十
    (2, "ひゃく"),  // 百
    (3, "せん"),    // 千
    (4, "まん"),    // 万
    (8, "おく"),    // 億
    (12, "ちょう"), // 兆
    (16, "けい"),   // 京
];
