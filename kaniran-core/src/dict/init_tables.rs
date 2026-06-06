//! Port of `ichiran/dict:init-tables` (`dict-load.lisp:7`).
//!
//! Empties the entry-package tables so JMdict data can be reloaded
//! into a clean schema.

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
