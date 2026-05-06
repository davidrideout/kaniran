//! Port of `ichiran/dict:*no-conj-data*` (`dict.lisp:329`).
//!
//! Per-seq marker for JMdict entries that have no rows in the
//! `conjugation` table — calculated negatively because the no-conj
//! set is much smaller than the conjugatable set and is more robust
//! when new conjugations are added (per upstream comment). Used as a
//! set: lookups go through [`super::no_conj_data::no_conj_data`],
//! which Lisp implements as
//! `(nth-value 1 (gethash seq (ensure :no-conj-data)))` — only key
//! presence matters, so the Rust value is a [`HashSet`].
//!
//! ## Storage
//!
//! Owned by [`KaniranContext::no_conj_data`]. [`build_no_conj_data`]
//! runs the upstream `defcache :no-conj-data` body — one SELECT
//! against `entry LEFT JOIN conjugation` — and is invoked once
//! during [`KaniranContext::from_url`]. After that the populated
//! set lives on the context for the life of the process; lookups
//! are direct field reads.

use crate::conn::kani_context::KaniranContext;
use sqlx::PgPool;
use std::collections::HashSet;

/// Borrow the no-conjugation-data seq registry off the context.
///
/// Named with the `_cache` suffix because the bare name
/// `no_conj_data` is reserved for the predicate function port at
/// `dict.lisp:339` (`super::no_conj_data::no_conj_data`).
pub fn no_conj_data_cache(ctx: &KaniranContext) -> &HashSet<i32> {
    &ctx.no_conj_data
}

/// Run the upstream `defcache :no-conj-data` body: query
/// `entry LEFT JOIN conjugation` for seqs whose `conjugation.seq`
/// is NULL (i.e. entries with no conjugation rows) and build the
/// set. Called from [`KaniranContext::from_url`].
pub async fn build_no_conj_data(pool: &PgPool) -> Result<HashSet<i32>, sqlx::Error> {
    let seqs: Vec<i32> = sqlx::query_scalar(
        "SELECT entry.seq FROM entry \
         LEFT JOIN conjugation c ON entry.seq = c.seq \
         WHERE c.seq IS NULL",
    )
    .fetch_all(pool)
    .await?;
    Ok(seqs.into_iter().collect())
}
