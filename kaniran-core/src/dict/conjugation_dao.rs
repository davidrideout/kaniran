//! Port of `ichiran/dict:conjugation` (`dict.lisp:238`).
//!
//! Row of the `public.conjugation` table — one conjugation link
//! recording that entry `seq` was derived from entry `seq_from`,
//! optionally via an intermediate entry `seq_via`.

use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct Conjugation {
    pub id: i32,
    pub seq: i32,
    pub seq_from: i32,
    pub seq_via: Option<i32>,
}

impl<'r> FromRow<'r, PgRow> for Conjugation {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Conjugation {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            seq_from: row.try_get("from")?,
            seq_via: row.try_get("via")?,
        })
    }
}
