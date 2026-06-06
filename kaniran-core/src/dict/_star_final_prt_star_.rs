//! Port of `ichiran/dict:*final-prt*` (`dict-errata.lisp:1182`).
//!
//! Seqs of words that only have meaning when they're the final
//! segment of a path.

pub static FINAL_PRT: &[i32] = &[
    2017770, // かい
    // 1008450 // では (commented out upstream)
    2425930, // なの
    // 2780660 // もの (commented out upstream)
    2130430, // け / っけ
    2029130, // ぞ
    2834812, // ぜ
    2718360, // がな
    2201380, // わい
    2722170, // のう
    2751630, // かいな
];
