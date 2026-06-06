//! Port of `ichiran/dict:sense` (`dict.lisp:166`).
//!
//! Row of the `public.sense` table — one numbered meaning attached to
//! an entry, ordered within the entry by the `ord` column.

use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct Sense {
    pub id: i32,
    pub seq: i32,
    pub ord: i32,
}

impl<'r> FromRow<'r, PgRow> for Sense {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Sense {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            ord: row.try_get("ord")?,
        })
    }
}
