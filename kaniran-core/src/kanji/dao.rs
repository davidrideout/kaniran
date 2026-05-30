//! Row representations for the four kanjidic2 tables and the
//! kanjidic2.xml path constant. From `kanji.lisp:3-100`.

use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

/// `*kanjidic-path*` (`kanji.lisp:3`). Upstream `defvar` placeholder;
/// per-deployment shadowing or config overrides it before
/// `load-kanjidic` runs.
pub static KANJIDIC_PATH: &str = "e:/dump/kanjidic2.xml";

/// `kanji` (`kanji.lisp:10`). Maps 1:1 to `public.kanji`.
/// `radical_c` = Kangxi radical, `radical_n` = Nelson radical.
/// `grade` and `freq` are nullable per kanjidic2. `stat_common` and
/// `stat_irregular` are corpus-loader counters (`:initform 0`, no
/// `:initarg`).
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

/// `reading` (`kanji.lisp:42`). Maps 1:1 to `public.reading`.
/// `kanji_id` foreign-keys to `kanji.id`. Slot is named `type` in Lisp;
/// the `:reader` is `reading-type` and the Rust field follows the
/// reader. `suffixp`/`prefixp` mark attachment-only readings.
/// `stat_common` is a loader counter (`:initform 0`, no `:initarg`).
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

/// `okurigana` (`kanji.lisp:67`). Maps 1:1 to `public.okurigana`.
/// `reading_id` foreign-keys to `reading.id`.
#[derive(Debug, Clone)]
pub struct Okurigana {
    pub id: i32,
    pub reading_id: i32,
    pub text: String,
}

impl<'r> FromRow<'r, PgRow> for Okurigana {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Okurigana {
            id: row.try_get("id")?,
            reading_id: row.try_get("reading_id")?,
            text: row.try_get("text")?,
        })
    }
}

/// `meaning` (`kanji.lisp:83`). Maps 1:1 to `public.meaning`.
/// `kanji_id` foreign-keys to `kanji.id`.
#[derive(Debug, Clone)]
pub struct Meaning {
    pub id: i32,
    pub kanji_id: i32,
    pub text: String,
}

impl<'r> FromRow<'r, PgRow> for Meaning {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Meaning {
            id: row.try_get("id")?,
            kanji_id: row.try_get("kanji_id")?,
            text: row.try_get("text")?,
        })
    }
}
