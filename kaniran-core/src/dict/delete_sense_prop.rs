//! Port of `ichiran/dict:delete-sense-prop` (`dict-errata.lisp:138`).
//!
//! Removes every `sense-prop` row matching `(seq, tag, text)`.

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
