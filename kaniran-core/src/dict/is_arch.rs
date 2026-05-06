//! Port of `ichiran/dict:is-arch` (`dict.lisp:759`).
//!
//! True when `seq` is recorded as archaic. Lisp's
//! `(nth-value 1 (gethash seq cache))` reads only key presence.

use crate::conn::kani_context::KaniranContext;

pub fn is_arch(ctx: &KaniranContext, seq: i32) -> bool {
    ctx.is_arch.contains(&seq)
}
