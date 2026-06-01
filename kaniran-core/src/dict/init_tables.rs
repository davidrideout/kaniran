//! Port of `ichiran/dict:init-tables` (`dict-load.lisp:7`).
//!
//! Diverges from upstream's per-DAO `drop-table` + `create-table` pair:
//! the Rust port assumes the schema is already in place (applied
//! externally per a fresh-DB-per-run model) and just empties the
//! entry-package tables atomically with one `TRUNCATE ... RESTART
//! IDENTITY CASCADE`. Reset of the SERIAL sequences mirrors the
//! upstream drop-and-recreate side effect.

use crate::conn::kani_context::KaniranContext;

pub const TABLE_NAMES: &[&str] = &[
    "entry",
    "kanji_text",
    "kana_text",
    "sense",
    "gloss",
    "sense_prop",
    "conjugation",
    "conj_prop",
    "conj_source_reading",
    "restricted_readings",
];

pub async fn init_tables(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    sqlx::query(
        "TRUNCATE entry, kanji_text, kana_text, sense, gloss, sense_prop, \
         conjugation, conj_prop, conj_source_reading, restricted_readings \
         RESTART IDENTITY CASCADE",
    )
    .execute(&ctx.pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Issues DDL (TRUNCATE), so refuses to touch any database that
    //! hasn't been explicitly named via `KANIRAN_TEST_DATABASE_URL`.
    //! Requires `--test-threads=1` since parallel runs share the same
    //! database.
    use super::*;
    use crate::conn::kani_context::KaniranContext;

    #[tokio::test]
    #[ignore = "DDL test; requires KANIRAN_TEST_DATABASE_URL"]
    async fn init_tables_is_idempotent() {
        let ctx = KaniranContext::pool_only_test_ctx().await;
        init_tables(&ctx).await.expect("first run");
        init_tables(&ctx).await.expect("second run must be idempotent");
    }
}
