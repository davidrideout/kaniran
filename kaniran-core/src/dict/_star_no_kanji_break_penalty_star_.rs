//! Port of `ichiran/dict:*no-kanji-break-penalty*` (`dict-errata.lisp:1214`).
//!
//! Seqs of words that are exempt from the kanji-break penalty.

pub static NO_KANJI_BREAK_PENALTY: &[i32] = &[
    1169870, // 飲む
    1198360, // 会議
    1277450, // 好き
    2028980, // で
    1423000, // 着る
    1164690, // 一段
    1587040, // 言う
    2827864, // なので
];
