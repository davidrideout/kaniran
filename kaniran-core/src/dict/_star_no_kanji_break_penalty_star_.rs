//! Port of `ichiran/dict:*no-kanji-break-penalty*` (`dict-errata.lisp:1214`).
//!
//! ```lisp
//! (defparameter *no-kanji-break-penalty*
//!   '(1169870 ;; 飲む
//!     1198360 ;; 会議
//!     1277450 ;; 好き
//!     2028980 ;; で
//!     1423000 ;; 着る
//!     1164690 ;; 一段
//!     1587040 ;; 言う
//!     2827864 ;; なので
//!     )
//!   "Words that get no kanji break penalty")
//! ```
//!
//! Consulted by `kanji-break-penalty` at `dict.lisp:709` as
//! `(intersection (getf info :seq-set) *no-kanji-break-penalty*)` —
//! when the candidate's `seq-set` intersects this list, the function
//! returns `score` unchanged (skips the kanji-break penalty branch).

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
