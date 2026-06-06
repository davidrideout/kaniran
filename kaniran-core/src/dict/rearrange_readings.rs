//! Port of `ichiran/dict:rearrange-readings` (`dict-errata.lisp:229`).
//!
//! Reassigns `ord` so every row whose `text` starts with `prefix`
//! lands first (0..offset) and the rest follow (offset..n), preserving
//! the original ascending-`ord` order inside each group.

use super::kani_reading_table::KaniReadingTable;
use crate::conn::kani_context::KaniranContext;
use sqlx::Row;

pub async fn rearrange_readings(
    ctx: &KaniranContext,
    seq: i32,
    table: KaniReadingTable,
    prefix: &str,
) -> Result<(), sqlx::Error> {
    let tname = table.table_name();
    // dict-errata.lisp:232-234 (query (:select (:count 'id) … (:like 'text (:|| prefix "%"))) :single)
    let pattern = format!("{prefix}%");
    let offset: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(id) FROM {tname} WHERE seq = $1 AND text LIKE $2",
    ))
    .bind(seq)
    .bind(&pattern)
    .fetch_one(&ctx.pool)
    .await?;
    // dict-errata.lisp:235 (with cnt1 = -1 and cnt2 = (1- offset))
    let mut cnt1: i32 = -1;
    let mut cnt2: i32 = (offset as i32) - 1;
    // dict-errata.lisp:236 (select-dao table (:= 'seq seq) 'ord) — sorted by ord asc
    let rows = sqlx::query(&format!(
        "SELECT id, text FROM {tname} WHERE seq = $1 ORDER BY ord",
    ))
    .bind(seq)
    .fetch_all(&ctx.pool)
    .await?;
    for row in rows {
        let id: i32 = row.try_get("id")?;
        let text: String = row.try_get("text")?;
        // dict-errata.lisp:237-238 (if (alexandria:starts-with-subseq prefix (text kt)) (incf cnt1) (incf cnt2))
        let new_ord = if text.starts_with(prefix) {
            cnt1 += 1;
            cnt1
        } else {
            cnt2 += 1;
            cnt2
        };
        // dict-errata.lisp:239 (setf (slot-value kt 'ord) new-ord) (update-dao kt)
        sqlx::query(&format!("UPDATE {tname} SET ord = $1 WHERE id = $2"))
            .bind(new_ord)
            .bind(id)
            .execute(&ctx.pool)
            .await?;
    }
    Ok(())
}
