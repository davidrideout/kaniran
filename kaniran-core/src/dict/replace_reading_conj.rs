//! Port of `ichiran/dict:replace-reading-conj` (`dict-errata.lisp:60`).
//!
//! For `seq` and every entry conjugated from it, rewrites rows of
//! `table` whose `text` starts with `prefix_from` to start with
//! `prefix_to` instead, then [`reset_readings`] across the touched
//! seqs.

use super::kani_reading_table::KaniReadingTable;
use super::reset_readings::reset_readings;
use crate::conn::kani_context::KaniranContext;

pub async fn replace_reading_conj(
    ctx: &KaniranContext,
    seq: i32,
    table: KaniReadingTable,
    prefix_from: &str,
    prefix_to: &str,
) -> Result<(), sqlx::Error> {
    let mut seqs: Vec<i32> = vec![seq];
    let conj_seqs: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT seq FROM conjugation WHERE \"from\" = $1",
    )
    .bind(seq)
    .fetch_all(&ctx.pool)
    .await?;
    seqs.extend(conj_seqs);
    let tname = table.table_name();
    let like_pat = format!("{}%", prefix_from);
    let rows: Vec<(i32, i32, String)> = sqlx::query_as(&format!(
        "SELECT id, seq, text FROM {} WHERE seq = ANY($1) AND text LIKE $2 ORDER BY seq",
        tname
    ))
    .bind(&seqs)
    .bind(&like_pat)
    .fetch_all(&ctx.pool)
    .await?;
    let prefix_from_chars = prefix_from.chars().count();
    let mut to_update: Vec<i32> = Vec::new();
    for (id, row_seq, text) in &rows {
        let tail: String = text.chars().skip(prefix_from_chars).collect();
        let new_text = format!("{}{}", prefix_to, tail);
        sqlx::query(&format!("UPDATE {} SET text = $1 WHERE id = $2", tname))
            .bind(&new_text)
            .bind(id)
            .execute(&ctx.pool)
            .await?;
        to_update.push(*row_seq);
    }
    if !to_update.is_empty() {
        reset_readings(ctx, &to_update).await?;
    }
    Ok(())
}
