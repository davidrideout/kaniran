//! Port of `ichiran/kanji:init-tables` (`kanji.lisp:100`).
//!
//! Empties the four kanji tables (kanji, reading, okurigana, meaning)
//! and resets their identity sequences before a corpus load.

use crate::conn::kani_context::KaniranContext;

pub const TABLE_NAMES: &[&str] = &["kanji", "reading", "okurigana", "meaning"];

pub async fn init_tables(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    sqlx::query(
        "TRUNCATE kanji, reading, okurigana, meaning RESTART IDENTITY CASCADE",
    )
    .execute(&ctx.pool)
    .await?;
    Ok(())
}
