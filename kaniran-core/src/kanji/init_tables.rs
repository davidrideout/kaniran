//! Port of `ichiran/kanji:init-tables` (`kanji.lisp:100`).
//!
//! Diverges from upstream's per-DAO `drop-table` + `create-table` pair:
//! the Rust port assumes the schema is already in place (applied
//! externally per a fresh-DB-per-run model) and just empties the four
//! kanji tables atomically with one `TRUNCATE ... RESTART IDENTITY
//! CASCADE`. Reset of the SERIAL sequences mirrors the upstream
//! drop-and-recreate side effect.

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
