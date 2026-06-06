//! Port of `ichiran/dict:*skip-counter-ids*` (`dict-counters.lisp:315`).
//!
//! Seqs JMdict tags as `pos=ctr` but that are excluded from the
//! counter cache (ambiguous readings, mahjong terms with no canonical
//! reading).

pub static SKIP_COUNTER_IDS: &[i32] = &[
    2426510, // 一個当り
    2220370, // 歳 （とせ）
    2248360, // 入 （しお）
    2423450, // 差し
    2671670, // 幅 （の）
    2735690, // 種 （くさ）
    2838543, // 杯 （はた）
    // mahjong stuff - need some research on how to say these
    2249290, // 荘
    2833260, // 翻
    2833465, // 萬
    2833466, // 索
    2833467, // 筒
];
