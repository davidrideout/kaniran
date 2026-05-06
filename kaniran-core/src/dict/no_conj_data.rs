//! Port of `ichiran/dict:no-conj-data` (`dict.lisp:339`).
//!
//! Predicate over [`super::_star_no_conj_data_star_::no_conj_data_cache`]:
//! returns `true` when the seq is recorded in the cache (= the entry
//! has no rows in the `conjugation` table). The Lisp form
//! `(nth-value 1 (gethash seq cache))` reads only key presence, so
//! the Rust port collapses to [`HashMap::contains_key`].
//!
//! The cache is populated eagerly during
//! [`crate::conn::kani_context::KaniranContext::from_url`], so the
//! predicate is always answering against the real registry.

use crate::conn::kani_context::KaniranContext;

pub fn no_conj_data(ctx: &KaniranContext, seq: i32) -> bool {
    ctx.no_conj_data.contains(&seq)
}
