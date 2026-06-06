//! Port of `ichiran/dict:get-counter-ids` (`dict-counters.lisp:283`).
//!
//! Returns the sorted list of JMdict sequence numbers tagged
//! `pos=ctr` (counter words) on at least one of their senses.

use crate::conn::kani_context::KaniranContext;
use sqlx::Row;

pub async fn get_counter_ids(ctx: &KaniranContext) -> Result<Vec<i32>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT DISTINCT seq FROM sense_prop WHERE tag = 'pos' AND text = 'ctr'",
    )
    .fetch_all(&ctx.pool)
    .await?;
    let mut seqs: Vec<i32> = rows.into_iter().map(|r| r.get::<i32, _>("seq")).collect();
    seqs.sort();
    Ok(seqs)
}
