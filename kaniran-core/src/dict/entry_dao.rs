//! Port of `ichiran/dict:entry` (`dict.lisp:26`).
//!
//! Row representation of one JMdict entry, mapped 1:1 to the
//! `public.entry` Postgres table populated by ichiran's schema.
//! `seq` is the JMdict sequence number (primary key). `n_kanji` and
//! `n_kana` cache the row counts of the entry's `kanji_text` and
//! `kana_text` children — the upstream `recalc-entry-stats[-all]`
//! refreshes them after a corpus reload. `root_p` flags entries
//! retained as themselves through derivation pruning;
//! `primary_nokanji` flags kana-only headwords with no kanji form.
//!
//! The methods upstream defines on this class (`get-kana`,
//! `get-text`, `get-kanji`, `print-object`, `common`) belong to
//! generic functions that have their own symbol entries in the port
//! plan and land in their own files when those generics are ported.

use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct Entry {
    pub seq: i32,
    pub content: String,
    pub root_p: bool,
    pub n_kanji: i32,
    pub n_kana: i32,
    pub primary_nokanji: bool,
}

impl<'r> FromRow<'r, PgRow> for Entry {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Entry {
            seq: row.try_get("seq")?,
            content: row.try_get("content")?,
            root_p: row.try_get("root_p")?,
            n_kanji: row.try_get("n_kanji")?,
            n_kana: row.try_get("n_kana")?,
            primary_nokanji: row.try_get("primary_nokanji")?,
        })
    }
}
