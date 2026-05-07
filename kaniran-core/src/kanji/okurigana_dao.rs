//! Port of `ichiran/kanji:okurigana` (`kanji.lisp:67`).
//!
//! Row representation of one okurigana fragment attached to a
//! kun-yomi reading, mapped 1:1 to the `public.okurigana` Postgres
//! table populated by ichiran's schema. `reading_id` foreign-keys
//! to `reading.id`.

use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct Okurigana {
    pub id: i32,
    pub reading_id: i32,
    pub text: String,
}

impl<'r> FromRow<'r, PgRow> for Okurigana {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Okurigana {
            id: row.try_get("id")?,
            reading_id: row.try_get("reading_id")?,
            text: row.try_get("text")?,
        })
    }
}
