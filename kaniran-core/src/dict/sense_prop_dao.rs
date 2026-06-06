//! Port of `ichiran/dict:sense-prop` (`dict.lisp:197`).
//!
//! Row of the `public.sense_prop` table — one tagged property on a
//! sense, where `tag` is the property kind (`pos`, `stagk`, `misc`, …)
//! and `text` holds its value.

use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct SenseProp {
    pub id: i32,
    pub tag: String,
    pub sense_id: i32,
    pub text: String,
    pub ord: i32,
    pub seq: i32,
}

impl<'r> FromRow<'r, PgRow> for SenseProp {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(SenseProp {
            id: row.try_get("id")?,
            tag: row.try_get("tag")?,
            sense_id: row.try_get("sense_id")?,
            text: row.try_get("text")?,
            ord: row.try_get("ord")?,
            seq: row.try_get("seq")?,
        })
    }
}
