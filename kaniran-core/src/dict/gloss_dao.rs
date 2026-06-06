//! Port of `ichiran/dict:gloss` (`dict.lisp:178`).
//!
//! Row representation of one English gloss attached to a JMdict sense.

use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct Gloss {
    pub id: i32,
    pub sense_id: i32,
    pub text: String,
    pub ord: i32,
}

impl<'r> FromRow<'r, PgRow> for Gloss {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Gloss {
            id: row.try_get("id")?,
            sense_id: row.try_get("sense_id")?,
            text: row.try_get("text")?,
            ord: row.try_get("ord")?,
        })
    }
}
