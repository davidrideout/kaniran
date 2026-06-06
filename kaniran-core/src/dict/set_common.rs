//! Port of `ichiran/dict:set-common` (`dict-errata.lisp:168`).
//!
//! Updates the `common` field on every row in `table` matching
//! `(seq, text)` to `common` (`None` writes SQL NULL).

use super::kani_reading_table::KaniReadingTable;
use crate::conn::kani_context::KaniranContext;

pub async fn set_common(
    ctx: &KaniranContext,
    table: KaniReadingTable,
    seq: i32,
    text: &str,
    common: Option<i32>,
) -> Result<(), sqlx::Error> {
    let sql = format!(
        "UPDATE {} SET common = $1 WHERE seq = $2 AND text = $3",
        table.table_name()
    );
    sqlx::query(&sql)
        .bind(common)
        .bind(seq)
        .bind(text)
        .execute(&ctx.pool)
        .await?;
    Ok(())
}
