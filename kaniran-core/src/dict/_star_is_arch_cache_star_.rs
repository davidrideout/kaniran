//! Port of `ichiran/dict:*is-arch-cache*` (`dict.lisp:745`).
//!
//! Set of seqs whose every sense is tagged `arch`/`obsc`/`rare`, plus
//! every conjugation root whose `from` column points at such a seq.

use crate::conn::kani_context::KaniranContext;
use sqlx::PgPool;
use std::collections::HashSet;

pub fn is_arch_cache(ctx: &KaniranContext) -> &HashSet<i32> {
    &ctx.is_arch
}

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
