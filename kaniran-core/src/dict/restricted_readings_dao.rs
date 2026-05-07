//! Port of `ichiran/dict:restricted-readings` (`dict.lisp:221`).
//!
//! Row representation of one JMdict re_restr / ke_restr restriction,
//! mapped 1:1 to the `public.restricted_readings` Postgres table
//! populated by ichiran's schema. Each row links a kana `reading` to
//! the kanji `text` form within an entry (`seq`) it is allowed to
//! pair with — used to filter readings to the subset valid for a
//! given kanji surface.

use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct RestrictedReadings {
    pub id: i32,
    pub seq: i32,
    pub reading: String,
    pub text: String,
}

impl<'r> FromRow<'r, PgRow> for RestrictedReadings {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(RestrictedReadings {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            reading: row.try_get("reading")?,
            text: row.try_get("text")?,
        })
    }
}
