//! Port of `ichiran/dict:drop-extras` (`dict-load.lisp:196`).
//!
//! Wipes the rows added back on top of a raw JMdict load — every
//! conjugation, every conjugation property, every conj-source-reading,
//! and every non-root `entry` row.

use crate::conn::kani_context::KaniranContext;

pub async fn drop_extras(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM conj_prop").execute(&ctx.pool).await?;
    sqlx::query("DELETE FROM conj_source_reading")
        .execute(&ctx.pool)
        .await?;
    sqlx::query("DELETE FROM conjugation")
        .execute(&ctx.pool)
        .await?;
    sqlx::query("DELETE FROM entry WHERE NOT root_p")
        .execute(&ctx.pool)
        .await?;
    Ok(())
}
