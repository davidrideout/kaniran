//! Port of `ichiran/dict:add-sense` (`dict-errata.lisp:148`).
//!
//! Inserts a new sense at `(seq, ord)` plus its glosses, unless a
//! sense at `(seq, ord)` already exists.
//!
//! Diverges from the upstream lambda list `(seq ord &rest glosses)`
//! by:
//! - taking `&KaniranContext` for the database handle, replacing the
//!   upstream dynamic `*connection*` per
//!   [`crate::conn::kani_context`];
//! - representing the variadic `&rest glosses` as `&[&str]`.

use crate::conn::kani_context::KaniranContext;

pub async fn add_sense(
    ctx: &KaniranContext,
    seq: i32,
    ord: i32,
    glosses: &[&str],
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:149 (unless (select-dao 'sense (:and (:= 'seq seq) (:= 'ord ord))))
    let existing: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM sense WHERE seq = $1 AND ord = $2 LIMIT 1",
    )
    .bind(seq)
    .bind(ord)
    .fetch_optional(&ctx.pool)
    .await?;
    if existing.is_some() {
        return Ok(());
    }
    // dict-errata.lisp:150 (id (make-dao 'sense :seq seq :ord ord))
    let sense_id: i32 = sqlx::query_scalar(
        "INSERT INTO sense (seq, ord) VALUES ($1, $2) RETURNING id",
    )
    .bind(seq)
    .bind(ord)
    .fetch_one(&ctx.pool)
    .await?;
    // dict-errata.lisp:151-153 (loop for gord from 0 for gloss in glosses do (make-dao 'gloss …))
    for (gord, gloss) in glosses.iter().enumerate() {
        sqlx::query("INSERT INTO gloss (sense_id, text, ord) VALUES ($1, $2, $3)")
            .bind(sense_id)
            .bind(gloss)
            .bind(gord as i32)
            .execute(&ctx.pool)
            .await?;
    }
    Ok(())
}
