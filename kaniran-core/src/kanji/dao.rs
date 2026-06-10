use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

/// Port of `ichiran/kanji:kanji` (`kanji.lisp:10`).
///
/// Row representation of one kanjidic2 character record. `radical_c` is
/// the classical (Kangxi) radical number; `radical_n` is the Nelson
/// radical.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
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

/// Port of `ichiran/kanji:reading` (`kanji.lisp:42`).
///
/// Row representation of one kanjidic2 reading record, mapped 1:1
/// to the `public.reading` Postgres table populated by ichiran's
/// schema. `kanji_id` foreign-keys to `kanji.id`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
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

/// Port of `ichiran/kanji:okurigana` (`kanji.lisp:67`).
///
/// Row representation of one okurigana fragment attached to a
/// kun-yomi reading, mapped 1:1 to the `public.okurigana` Postgres
/// table populated by ichiran's schema. `reading_id` foreign-keys
/// to `reading.id`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
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

/// Port of `ichiran/kanji:meaning` (`kanji.lisp:83`).
///
/// Row representation of one English meaning attached to a kanjidic2
/// character, mapped 1:1 to the `public.meaning` Postgres table
/// populated by ichiran's schema. `kanji_id` foreign-keys to
/// `kanji.id`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
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
