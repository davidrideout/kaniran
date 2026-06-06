//! Port of `ichiran/dict:kana-text` (`dict.lisp:128`).
//!
//! Row representation of a kana reading of a JMdict entry.

use crate::dict::simple_text_class::SimpleText;
use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct KanaText {
    pub id: i32,
    pub seq: i32,
    pub text: String,
    pub ord: i32,
    pub common: Option<i32>,
    pub common_tags: String,
    pub conjugate_p: bool,
    pub nokanji: bool,
    pub best_kanji: Option<String>,
    pub state: SimpleText,
}

impl<'r> FromRow<'r, PgRow> for KanaText {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(KanaText {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            text: row.try_get("text")?,
            ord: row.try_get("ord")?,
            common: row.try_get("common")?,
            common_tags: row.try_get("common_tags")?,
            conjugate_p: row.try_get("conjugate_p")?,
            nokanji: row.try_get("nokanji")?,
            best_kanji: row.try_get("best_kanji")?,
            state: SimpleText::default(),
        })
    }
}
