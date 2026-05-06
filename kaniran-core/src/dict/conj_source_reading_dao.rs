//! Port of `ichiran/dict:conj-source-reading` (`dict.lisp:309`).
//!
//! Row representation of one (text, source-text) pair attached to a
//! [`Conjugation`] row — the rendered conjugated form (`text`) and
//! the dictionary surface form it derives from (`source-text`).
//! Mapped 1:1 to the `public.conj_source_reading` Postgres table.
//! Used by [`super::get_conj_data`] to populate the `src_map` slot
//! of [`super::conj_data_struct::ConjData`].
//!
//! [`Conjugation`]: super::conjugation_dao::Conjugation

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
