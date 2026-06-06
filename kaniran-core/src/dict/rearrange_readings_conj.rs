//! Port of `ichiran/dict:rearrange-readings-conj` (`dict-errata.lisp:241`).
//!
//! Runs [`rearrange_readings`] for `seq`, then runs it again for every
//! distinct `conjugation.seq` whose `from = seq`.

use super::kani_reading_table::KaniReadingTable;
use super::rearrange_readings::rearrange_readings;
use crate::conn::kani_context::KaniranContext;

pub async fn rearrange_readings_conj(
    ctx: &KaniranContext,
    seq: i32,
    table: KaniReadingTable,
    prefix: &str,
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:242 (rearrange-readings seq table prefix)
    rearrange_readings(ctx, seq, table, prefix).await?;
    // dict-errata.lisp:243 (dolist (seq (query (:select 'seq :distinct :from 'conjugation :where (:= 'from seq)) :column)) …)
    let conj_seqs: Vec<i32> = sqlx::query_scalar(
        r#"SELECT DISTINCT seq FROM conjugation WHERE "from" = $1"#,
    )
    .bind(seq)
    .fetch_all(&ctx.pool)
    .await?;
    for conj_seq in conj_seqs {
        // dict-errata.lisp:244 (rearrange-readings seq table prefix)
        rearrange_readings(ctx, conj_seq, table, prefix).await?;
    }
    Ok(())
}
