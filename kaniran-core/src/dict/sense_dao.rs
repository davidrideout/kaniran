//! Port of `ichiran/dict:sense` (`dict.lisp:166`).
//!
//! Row representation of a single JMdict sense, mapped 1:1 to the
//! `public.sense` Postgres table populated by ichiran's schema. A
//! sense is a numbered meaning attached to an entry — the parent of
//! gloss rows (the English definitions) and `sense-prop` rows (the
//! tagged metadata such as `pos=ctr`). The ordering within an entry
//! is held in the `ord` column.

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
