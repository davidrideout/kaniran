//! Port of `ichiran/dict:conj-source-reading` (`dict.lisp:309`).
//!
//! Row of the `public.conj_source_reading` table — one (text,
//! source-text) pair giving a rendered conjugated form and the
//! dictionary surface form it derives from.

use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct ConjSourceReading {
    pub id: i32,
    pub conj_id: i32,
    pub text: String,
    pub source_text: String,
}

impl<'r> FromRow<'r, PgRow> for ConjSourceReading {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(ConjSourceReading {
            id: row.try_get("id")?,
            conj_id: row.try_get("conj_id")?,
            text: row.try_get("text")?,
            source_text: row.try_get("source_text")?,
        })
    }
}
