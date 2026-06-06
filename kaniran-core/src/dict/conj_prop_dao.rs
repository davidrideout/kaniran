//! Port of `ichiran/dict:conj-prop` (`dict.lisp:262`).
//!
//! Row of the `public.conj_prop` table — one tagged property attached
//! to a conjugation, the `(pos, conj-type, neg, fml)` quadruple naming
//! which conjugation form of which part-of-speech the row describes.

use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct ConjProp {
    pub id: i32,
    pub conj_id: i32,
    pub conj_type: i32,
    pub pos: String,
    pub neg: Option<bool>,
    pub fml: Option<bool>,
}

impl<'r> FromRow<'r, PgRow> for ConjProp {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(ConjProp {
            id: row.try_get("id")?,
            conj_id: row.try_get("conj_id")?,
            conj_type: row.try_get("conj_type")?,
            pos: row.try_get("pos")?,
            neg: row.try_get("neg")?,
            fml: row.try_get("fml")?,
        })
    }
}
