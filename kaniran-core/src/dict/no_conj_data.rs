//! Port of `ichiran/dict:no-conj-data` (`dict.lisp:339`).
//!
//! True when `seq` has no rows in the `conjugation` table.

use crate::conn::kani_context::KaniranContext;

pub fn no_conj_data(ctx: &KaniranContext, seq: i32) -> bool {
    ctx.no_conj_data.contains(&seq)
}
