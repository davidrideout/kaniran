//! Port of `ichiran/kanji:kanji` (`kanji.lisp:10`).
//!
//! Row representation of one kanjidic2 character record. `radical_c` is
//! the classical (Kangxi) radical number; `radical_n` is the Nelson
//! radical.

use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct Kanji {
    pub id: i32,
    pub text: String,
    pub radical_c: i32,
    pub radical_n: i32,
    pub grade: Option<i32>,
    pub strokes: i32,
    pub freq: Option<i32>,
    pub stat_common: i32,
    pub stat_irregular: i32,
}

impl<'r> FromRow<'r, PgRow> for Kanji {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Kanji {
            id: row.try_get("id")?,
            text: row.try_get("text")?,
            radical_c: row.try_get("radical_c")?,
            radical_n: row.try_get("radical_n")?,
            grade: row.try_get("grade")?,
            strokes: row.try_get("strokes")?,
            freq: row.try_get("freq")?,
            stat_common: row.try_get("stat_common")?,
            stat_irregular: row.try_get("stat_irregular")?,
        })
    }
}
