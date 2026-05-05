//! Port of `ichiran/dict:conjugation` (`dict.lisp:238`).
//!
//! Row representation of one conjugation link between JMdict entries,
//! mapped 1:1 to the `public.conjugation` Postgres table populated by
//! ichiran's schema. A row records that the entry with sequence
//! `seq` was derived from the entry with sequence `seq_from`,
//! optionally via an intermediate entry `seq_via`.
//!
//! The Lisp slot names are `from` and `via`; the upstream slot
//! readers — `seq-from` and `seq-via` — are the public surface every
//! caller uses. The Rust fields therefore take the reader names. The
//! Postgres column names remain `from` / `via`.

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
