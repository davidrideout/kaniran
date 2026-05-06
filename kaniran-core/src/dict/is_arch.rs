//! Port of `ichiran/dict:is-arch` (`dict.lisp:759`).
//!
//! Predicate over [`super::_star_is_arch_cache_star_::is_arch_cache`]:
//! returns `true` when the seq has been recorded as archaic. The
//! Lisp form `(nth-value 1 (gethash seq cache))` reads only key
//! presence, so the Rust port collapses to [`HashSet::contains`].
//!
//! The cache is populated eagerly during
//! [`crate::conn::kani_context::KaniranContext::from_url`].

use crate::conn::kani_context::KaniranContext;

pub fn is_arch(ctx: &KaniranContext, seq: i32) -> bool {
    ctx.is_arch.contains(&seq)
}
