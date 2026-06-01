//! Port of `ichiran/dict:set-primary-nokanji` (`dict-errata.lisp:224`).
//!
//! Looks up the entry by `seq` and writes `value` into its
//! `primary_nokanji` column.

use super::entry_dao::Entry;
use crate::conn::kani_context::KaniranContext;

pub async fn set_primary_nokanji(
    ctx: &KaniranContext,
    seq: i32,
    value: bool,
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:225 (get-dao 'entry seq)
    let mut entry: Entry = sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
        .bind(seq)
        .fetch_one(&ctx.pool)
        .await?;
    // dict-errata.lisp:226 (setf (slot-value entry 'primary-nokanji) value)
    entry.primary_nokanji = value;
    // dict-errata.lisp:227 (update-dao entry)
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
    Ok(())
}
