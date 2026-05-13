//! Port of `ichiran/dict:*skip-words*` (`dict-errata.lisp:1155`).
//!
//! "seq of words that aren't really words, like suffixes etc." —
//! `calc-score` (`dict.lisp:855`) returns 0 for any reading whose
//! `seq-set` intersects this list, removing the candidate from
//! segmentation.

pub static SKIP_WORDS: &[i32] = &[
    2822120, // ても良い
    2013800, // ちゃう
    2108590, // とく
    2029040, // ば
    2428180, // い
    2654250, // た
    2561100, // うまいな
    2210270, // ませんか
    2210710, // ましょうか
    2257550, // ない
    2210320, // ません
    2017560, // たい
    2394890, // とる
    2194000, // であ
    2568000, // れる/られる
    2537250, // しようとする
    2760890, // 三箱
    2831062, // てる
    2831063, // てく
    2029030, // ものの
    2568020, // せる
    900000,  // たそう
    2827357, // まう
];
