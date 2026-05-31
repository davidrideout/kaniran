//! Port of `ichiran/kanji:init-tables` (`kanji.lisp:100`).
//!
//! Drops the kanjidic2 tables

use crate::conn::kani_context::KaniranContext;
use sqlx::Executor;

pub const TABLE_NAMES: &[&str] = &["kanji", "reading", "okurigana", "meaning"];

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
