//! Port of `ichiran/dict:sense-prop` (`dict.lisp:197`).
//!
//! Row representation of one tagged property attached to a sense
//! ("part of speech is counter", "applies only to reading X", etc.),
//! mapped 1:1 to the `public.sense_prop` Postgres table populated by
//! ichiran's schema. The same `(seq, sense_id)` pair gathers all
//! properties for a single sense — `tag` discriminates the property
//! kind (`pos`, `stagk`, `stagr`, `misc`, `field`, etc.) and `text`
//! holds the property value.

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
