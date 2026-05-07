//! Port of `ichiran/kanji:reading` (`kanji.lisp:42`).
//!
//! Row representation of one kanjidic2 reading record, mapped 1:1
//! to the `public.reading` Postgres table populated by ichiran's
//! schema. `kanji_id` foreign-keys to `kanji.id`. `reading_type`
//! holds the kanjidic2 reading category (`ja_on`, `ja_kun`, etc.);
//! the slot is named `type` in Lisp, but the upstream `:reader` is
//! `reading-type` and the Rust field follows the reader. `suffixp`
//! and `prefixp` mark readings that only attach as suffixes or
//! prefixes (default `nil`). `stat_common` is a read-only counter
//! maintained by the corpus loader; upstream `:initform 0` and has
//! no `:initarg`.

use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct Reading {
    pub id: i32,
    pub kanji_id: i32,
    pub reading_type: String,
    pub text: String,
    pub suffixp: bool,
    pub prefixp: bool,
    pub stat_common: i32,
}

impl<'r> FromRow<'r, PgRow> for Reading {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Reading {
            id: row.try_get("id")?,
            kanji_id: row.try_get("kanji_id")?,
            reading_type: row.try_get("type")?,
            text: row.try_get("text")?,
            suffixp: row.try_get("suffixp")?,
            prefixp: row.try_get("prefixp")?,
            stat_common: row.try_get("stat_common")?,
        })
    }
}
