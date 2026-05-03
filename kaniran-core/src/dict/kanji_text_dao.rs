//! Port of `ichiran/dict:kanji-text` (`dict.lisp:86`).
//!
//! Row representation of a kanji writing of a JMdict entry, mapped 1:1
//! to the `public.kanji_text` Postgres table populated by ichiran's
//! schema. The 9 columns (id, seq, text, ord, common, common_tags,
//! conjugate_p, nokanji, best_kana) are persisted; the `state` field
//! holds the two runtime-only slots inherited from
//! [`crate::dict::simple_text_class::SimpleText`] and is reset to its
//! `Default` on every `FromRow` decode.
//!
//! `FromRow` is implemented by hand rather than derived because the
//! `state` field has no DB counterpart and the derive macro requires
//! every field's type to satisfy `Decode + Type` even when
//! `#[sqlx(default)]` is set.

use crate::dict::simple_text_class::SimpleText;
use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct KanjiText {
    pub id: i32,
    pub seq: i32,
    pub text: String,
    pub ord: i32,
    pub common: Option<i32>,
    pub common_tags: String,
    pub conjugate_p: bool,
    pub nokanji: bool,
    pub best_kana: Option<String>,
    pub state: SimpleText,
}

impl<'r> FromRow<'r, PgRow> for KanjiText {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(KanjiText {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            text: row.try_get("text")?,
            ord: row.try_get("ord")?,
            common: row.try_get("common")?,
            common_tags: row.try_get("common_tags")?,
            conjugate_p: row.try_get("conjugate_p")?,
            nokanji: row.try_get("nokanji")?,
            best_kana: row.try_get("best_kana")?,
            state: SimpleText::default(),
        })
    }
}
