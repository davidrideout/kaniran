//! Port of `ichiran/dict:init-tables` (`dict-load.lisp:7`).
//!
//! Drops the entry-package tables (in reverse declaration order with
//! `CASCADE` so any FKs pointing at them from other-package tables
//! tear down cleanly) and re-creates them. The DDL is the schema
//! [`db/schema.sql`](../../../../db/schema.sql), a `pg_dump
//! --schema-only --clean --if-exists` of the live ichiran database;
//! statements not naming any of [`TABLE_NAMES`] are skipped so calling
//! `dict::init_tables` leaves the kanji-package tables untouched
//! (mirrors the upstream per-package boundary).


use crate::conn::kani_context::KaniranContext;
use sqlx::Executor;

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

const SCHEMA_SQL: &str = include_str!("../../../db/schema.sql");

pub async fn init_tables(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    for name in TABLE_NAMES.iter().rev() {
        let drop_sql = format!("DROP TABLE IF EXISTS {name} CASCADE");
        ctx.pool.execute(drop_sql.as_str()).await?;
    }
    for statement in
        crate::kani_schema_filter::iter_relevant_statements(SCHEMA_SQL, TABLE_NAMES)
    {
        ctx.pool.execute(sqlx::raw_sql(&statement)).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! These tests issue DDL — `DROP TABLE` / `CREATE TABLE` — so they
    //! refuse to touch any database that hasn't been explicitly named
    //! via `KANIRAN_TEST_DATABASE_URL`, also rejects URLs
    //! equal to `DATABASE_URL` (the production handle).
    //!
    //! Requires:
    //! `--test-threads=1` since parallel runs share the same database
    use super::*;
    use crate::conn::kani_context::KaniranContext;

    #[tokio::test]
    #[ignore = "DDL test; requires KANIRAN_TEST_DATABASE_URL"]
    async fn init_tables_is_idempotent_via_drop() {
        let ctx = KaniranContext::pool_only_test_ctx().await;
        init_tables(&ctx).await.expect("first run");
        init_tables(&ctx).await.expect("second run should drop+recreate");
    }
}
