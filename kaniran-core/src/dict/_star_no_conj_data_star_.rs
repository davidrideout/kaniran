//! Port of `ichiran/dict:*no-conj-data*` (`dict.lisp:329`).
//!
//! Set of seqs for JMdict entries that have no rows in the
//! `conjugation` table.

use crate::conn::kani_context::KaniranContext;
use sqlx::PgPool;
use std::collections::HashSet;

/// `_cache` suffix because the bare name `no_conj_data` is reserved
/// for the predicate function port at `dict.lisp:339`
/// ([`super::no_conj_data::no_conj_data`]).
pub fn no_conj_data_cache(ctx: &KaniranContext) -> &HashSet<i32> {
    &ctx.no_conj_data
}

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
