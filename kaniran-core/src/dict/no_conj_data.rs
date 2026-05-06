//! Port of `ichiran/dict:no-conj-data` (`dict.lisp:339`).
//!
//! True when `seq` has no rows in the `conjugation` table. Lisp's
//! `(nth-value 1 (gethash seq cache))` reads only key presence, so
//! the Rust port is a `HashSet` membership check.

use crate::conn::kani_context::KaniranContext;

pub fn no_conj_data(ctx: &KaniranContext, seq: i32) -> bool {
    ctx.no_conj_data.contains(&seq)
}
