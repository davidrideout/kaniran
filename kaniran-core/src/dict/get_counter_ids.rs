//! Port of `ichiran/dict:get-counter-ids` (`dict-counters.lisp:283`).
//!
//! Returns the sorted list of JMdict sequence numbers tagged
//! `pos=ctr` (counter words) on at least one of their senses. The
//! result is the input set the counter cache draws from before
//! [`crate::dict::_star_extra_counter_ids_star_::EXTRA_COUNTER_IDS`]
//! is added and
//! [`crate::dict::_star_skip_counter_ids_star_::SKIP_COUNTER_IDS`]
//! is removed.
//!
//! Diverges from the upstream lambda list `()` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per the [`crate::conn::kani_context`]
//! module doc.

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
