//! Port of `ichiran/dict:delete-conjugation` (`dict-errata.lisp:198`).
//!
//! Drops every `conjugation` row from `from` to `seq` via `via`, plus
//! its `conj-prop` and `conj-source-reading` children, then drops the
//! target `entry` itself unless it's a root entry or still has other
//! conjugations.

use super::entry_dao::Entry;
use crate::conn::kani_context::KaniranContext;

pub async fn delete_conjugation(
    ctx: &KaniranContext,
    seq: i32,
    from: i32,
    via: Option<i32>,
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:199-203 (query-dao 'conjugation (:select '* :from 'conjugation :where …))
    // `:===` is null-safe equality; map to `IS NOT DISTINCT FROM`.
    let conj_ids: Vec<i32> = sqlx::query_scalar(
        r#"SELECT id FROM conjugation
           WHERE seq = $1 AND "from" = $2 AND via IS NOT DISTINCT FROM $3"#,
    )
    .bind(seq)
    .bind(from)
    .bind(via)
    .fetch_all(&ctx.pool)
    .await?;
    // dict-errata.lisp:204 (entry (get-dao 'entry seq))
    let entry: Entry = sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
        .bind(seq)
        .fetch_one(&ctx.pool)
        .await?;
    // dict-errata.lisp:205 (when conj …) — bail when no matching rows.
    if conj_ids.is_empty() {
        return Ok(());
    }
    // dict-errata.lisp:207-209 (delete-entry (not (or (root-p entry) (select-dao 'conjugation …))))
    let other_conj: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM conjugation WHERE seq = $1 AND NOT (id = ANY($2)) LIMIT 1",
    )
    .bind(seq)
    .bind(&conj_ids)
    .fetch_optional(&ctx.pool)
    .await?;
    let delete_entry = !(entry.root_p || other_conj.is_some());
    // dict-errata.lisp:211-213 (query (:delete-from …))
    sqlx::query("DELETE FROM conj_prop WHERE conj_id = ANY($1)")
        .bind(&conj_ids)
        .execute(&ctx.pool)
        .await?;
    sqlx::query("DELETE FROM conj_source_reading WHERE conj_id = ANY($1)")
        .bind(&conj_ids)
        .execute(&ctx.pool)
        .await?;
    sqlx::query("DELETE FROM conjugation WHERE id = ANY($1)")
        .bind(&conj_ids)
        .execute(&ctx.pool)
        .await?;
    // dict-errata.lisp:214-215 (when delete-entry (delete-dao entry))
    if delete_entry {
        sqlx::query("DELETE FROM entry WHERE seq = $1")
            .bind(seq)
            .execute(&ctx.pool)
            .await?;
    }
    Ok(())
}
