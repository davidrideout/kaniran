//! Port of `ichiran/dict:*skip-counter-ids*` (`dict-counters.lisp:315`).
//!
//! JMdict seqs that JMdict tags with `pos=ctr` but the project
//! excludes from the counter cache. Two reasons:
//! - Ambiguous or non-canonical readings that need disambiguation
//!   (歳=とせ, 入=しお, 種=くさ, 杯=はた, etc.).
//! - Mahjong terms with no canonical Japanese reading (荘, 翻,
//!   萬, 索, 筒) — flagged in the upstream comment as needing
//!   research.
//!
//! Subtracted from the union of the SQL "pos=ctr" sweep and
//! [`crate::dict::_star_extra_counter_ids_star_::EXTRA_COUNTER_IDS`]
//! by `get-counter-readings`.

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
