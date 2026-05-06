//! Port of `ichiran/dict:*is-arch-cache*` (`dict.lisp:745`).
//!
//! Per-seq marker for entries whose every sense is tagged
//! `arch` / `obsc` / `rare`, plus every conjugation root whose
//! `from` column points at such a seq. Used as a set: lookups go
//! through [`super::is_arch::is_arch`], which Lisp implements as
//! `(nth-value 1 (gethash seq (ensure :is-arch)))` — only key
//! presence matters, so the Rust value is a [`HashSet`].
//!
//! ## Storage
//!
//! Owned by [`KaniranContext::is_arch`]. [`build_is_arch`] runs the
//! upstream `defcache :is-arch` body — two queries (a misc-tag
//! GROUP BY having every-not-null check, then a conjugation-root
//! pull) followed by an in-memory union — and is invoked once
//! during [`KaniranContext::from_url`]. After that the populated
//! set lives on the context for the life of the process.
//!
//! ## SQL
//!
//! Upstream uses `:in (:set a1)` for the second query; we pass the
//! first result as a Postgres array via `= ANY($1)`. Equivalent
//! behavior under the same schema. `from` is a SQL reserved word so
//! it stays double-quoted.

use crate::conn::kani_context::KaniranContext;
use sqlx::PgPool;
use std::collections::HashSet;

/// Borrow the archaic-seq registry off the context.
pub fn is_arch_cache(ctx: &KaniranContext) -> &HashSet<i32> {
    &ctx.is_arch
}

/// Run the upstream `defcache :is-arch` body and return the
/// populated set. Called from [`KaniranContext::from_url`].
pub async fn build_is_arch(pool: &PgPool) -> Result<HashSet<i32>, sqlx::Error> {
    let a1: Vec<i32> = sqlx::query_scalar(
        "SELECT sense.seq FROM sense \
         LEFT JOIN sense_prop sp \
                ON sp.sense_id = sense.id \
               AND sp.tag = 'misc' \
               AND sp.text IN ('arch', 'obsc', 'rare') \
         GROUP BY sense.seq \
         HAVING bool_and(sp.id IS NOT NULL)",
    )
    .fetch_all(pool)
    .await?;
    let a2: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT seq FROM conjugation WHERE \"from\" = ANY($1)",
    )
    .bind(&a1)
    .fetch_all(pool)
    .await?;
    let mut set: HashSet<i32> = a1.into_iter().collect();
    set.extend(a2);
    Ok(set)
}
