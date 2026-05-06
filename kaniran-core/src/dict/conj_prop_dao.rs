//! Port of `ichiran/dict:conj-prop` (`dict.lisp:262`).
//!
//! Row representation of one tagged property attached to a
//! [`Conjugation`] row — the `(pos, conj-type, neg, fml)` quadruple
//! that identifies which conjugation form (past plain, negative
//! formal, causative-passive, etc.) of which part-of-speech the row
//! describes. Mapped 1:1 to the `public.conj_prop` Postgres table.
//!
//! The Lisp slot names `neg` and `fml` collide with predicate-style
//! readers `conj-neg` / `conj-fml`; the Postgres column names remain
//! `neg` / `fml`. Both are nullable booleans (`(or db-null boolean)`)
//! because the upstream JMdict pipeline records "unspecified" — those
//! land here as [`Option::None`].
//!
//! [`Conjugation`]: super::conjugation_dao::Conjugation

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
