//! Port of `ichiran/kanji:meaning` (`kanji.lisp:83`).
//!
//! Row representation of one English meaning attached to a kanjidic2
//! character, mapped 1:1 to the `public.meaning` Postgres table
//! populated by ichiran's schema. `kanji_id` foreign-keys to
//! `kanji.id`.

use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct Meaning {
    pub id: i32,
    pub kanji_id: i32,
    pub text: String,
}

impl<'r> FromRow<'r, PgRow> for Meaning {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Meaning {
            id: row.try_get("id")?,
            kanji_id: row.try_get("kanji_id")?,
            text: row.try_get("text")?,
        })
    }
}
