//! Port of `ichiran/dict:delete-reading` (`dict-errata.lisp:76`).
//!
//! Deletes every row of the entry's kana or kanji table whose `text`
//! equals `reading`, decrements the entry's matching counter,
//! renumbers survivors so `ord` is 0-based and contiguous, then calls
//! [`reset_readings`].

use super::entry_dao::Entry;
use super::kani_reading_table::KaniReadingTable;
use super::reset_readings::reset_readings;
use crate::characters::char_class::CharClass;
use crate::characters::char_class::test_word;
use crate::conn::kani_context::KaniranContext;
use sqlx::Row;

pub async fn delete_reading(
    ctx: &KaniranContext,
    seq: i32,
    reading: &str,
    table: Option<KaniReadingTable>,
) -> Result<(), sqlx::Error> {
    let is_kana = test_word(reading, CharClass::Kana);
    let table = table.unwrap_or(if is_kana {
        KaniReadingTable::Kana
    } else {
        KaniReadingTable::Kanji
    });
    let mut entry: Entry =
        sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
            .bind(seq)
            .fetch_one(&ctx.pool)
            .await?;
    let tname = table.table_name();
    let to_delete: Vec<i32> = sqlx::query(&format!(
        "SELECT id FROM {} WHERE seq = $1 AND text = $2",
        tname
    ))
    .bind(seq)
    .bind(reading)
    .fetch_all(&ctx.pool)
    .await?
    .into_iter()
    .map(|row| row.get::<i32, _>("id"))
    .collect();
    if !to_delete.is_empty() {
        let mut deleted: i32 = 0;
        for id in &to_delete {
            sqlx::query(&format!("DELETE FROM {} WHERE id = $1", tname))
                .bind(id)
                .execute(&ctx.pool)
                .await?;
            deleted += 1;
        }
        match table {
            KaniReadingTable::Kana => entry.n_kana -= deleted,
            KaniReadingTable::Kanji => entry.n_kanji -= deleted,
        }
        sqlx::query(
            "UPDATE entry SET content = $2, root_p = $3, n_kanji = $4, \
             n_kana = $5, primary_nokanji = $6 WHERE seq = $1",
        )
        .bind(entry.seq)
        .bind(&entry.content)
        .bind(entry.root_p)
        .bind(entry.n_kanji)
        .bind(entry.n_kana)
        .bind(entry.primary_nokanji)
        .execute(&ctx.pool)
        .await?;
        let survivor_ids: Vec<i32> = sqlx::query(&format!(
            "SELECT id FROM {} WHERE seq = $1 ORDER BY ord",
            tname
        ))
        .bind(seq)
        .fetch_all(&ctx.pool)
        .await?
        .into_iter()
        .map(|row| row.get::<i32, _>("id"))
        .collect();
        for (new_ord, id) in survivor_ids.iter().enumerate() {
            sqlx::query(&format!("UPDATE {} SET ord = $1 WHERE id = $2", tname))
                .bind(new_ord as i32)
                .bind(id)
                .execute(&ctx.pool)
                .await?;
        }
        reset_readings(ctx, &[seq]).await?;
    }
    Ok(())
}
