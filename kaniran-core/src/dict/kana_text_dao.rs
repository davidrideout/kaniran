//! Port of `ichiran/dict:kana-text` (`dict.lisp:128`).
//!
//! Row representation of a kana reading of a JMdict entry, mapped 1:1
//! to the `public.kana_text` Postgres table populated by ichiran's
//! schema. Identical column shape to
//! [`crate::dict::kanji_text_dao::KanjiText`] except for the asymmetric
//! cross-reference column `best_kanji` (vs `best_kana` on the kanji
//! row). The `state` field holds the two runtime-only slots inherited
//! from [`crate::dict::simple_text_class::SimpleText`] and is reset to
//! its `Default` on every `FromRow` decode.
//!
//! `FromRow` is implemented by hand rather than derived because the
//! `state` field has no DB counterpart and the derive macro requires
//! every field's type to satisfy `Decode + Type` even when
//! `#[sqlx(default)]` is set.

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
