//! Port of `ichiran/kanji:reading` (`kanji.lisp:42`).
//!
//! Row representation of one kanjidic2 reading record, mapped 1:1
//! to the `public.reading` Postgres table populated by ichiran's
//! schema. `kanji_id` foreign-keys to `kanji.id`.

use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct Reading {
    pub id: i32,
    pub kanji_id: i32,
    pub reading_type: String,
    pub text: String,
    pub suffixp: bool,
    pub prefixp: bool,
    pub stat_common: i32,
}

impl<'r> FromRow<'r, PgRow> for Reading {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Reading {
            id: row.try_get("id")?,
            kanji_id: row.try_get("kanji_id")?,
            reading_type: row.try_get("type")?,
            text: row.try_get("text")?,
            suffixp: row.try_get("suffixp")?,
            prefixp: row.try_get("prefixp")?,
            stat_common: row.try_get("stat_common")?,
        })
    }
}
