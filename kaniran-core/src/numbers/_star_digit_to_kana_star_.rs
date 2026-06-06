//! Port of `ichiran/numbers:*digit-to-kana*` (`numbers.lisp:25`).
//!
//! Reading for each digit `0..=9` in hiragana. Indexed by digit value;
//! `DIGIT_TO_KANA[n]` is the reading of `n`.

pub const DIGIT_TO_KANA: &[&str] = &[
    "れい",  // 0
    "いち",  // 1
    "に",    // 2
    "さん",  // 3
    "よん",  // 4
    "ご",    // 5
    "ろく",  // 6
    "なな",  // 7
    "はち",  // 8
    "きゅう", // 9
];
