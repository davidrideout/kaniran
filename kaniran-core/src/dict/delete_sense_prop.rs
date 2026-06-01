//! Port of `ichiran/dict:delete-sense-prop` (`dict-errata.lisp:138`).
//!
//! Removes every `sense-prop` row matching `(seq, tag, text)`.
//!
//! Diverges from the upstream lambda list `(seq tag text)` only by
//! taking `&KaniranContext` for the database handle, replacing the
//! upstream dynamic `*connection*` per [`crate::conn::kani_context`].

use crate::conn::kani_context::KaniranContext;

pub async fn delete_sense_prop(
    ctx: &KaniranContext,
    seq: i32,
    tag: &str,
    text: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sense_prop WHERE seq = $1 AND tag = $2 AND text = $3")
        .bind(seq)
        .bind(tag)
        .bind(text)
        .execute(&ctx.pool)
        .await?;
    Ok(())
}
