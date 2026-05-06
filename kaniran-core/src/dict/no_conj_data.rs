//! Port of `ichiran/dict:no-conj-data` (`dict.lisp:339`).
//!
//! Predicate over [`super::_star_no_conj_data_star_::no_conj_data_cache`]:
//! returns `true` when the seq is recorded in the cache (= the entry
//! has no rows in the `conjugation` table). The Lisp form
//! `(nth-value 1 (gethash seq cache))` reads only key presence, so
//! the Rust port collapses to [`HashMap::contains_key`].
//!
//! The cache is populated lazily by
//! [`super::_star_no_conj_data_star_::init_no_conj_data_cache`] —
//! callers that depend on `no_conj_data` being correct must await
//! that init before the first call. Until then, this returns
//! `false` for every seq (cache uninitialised).

use super::_star_no_conj_data_star_::no_conj_data_cache;

pub fn no_conj_data(seq: i32) -> bool {
    no_conj_data_cache()
        .map(|m| m.contains_key(&seq))
        .unwrap_or(false)
}
