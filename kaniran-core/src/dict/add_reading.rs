//! Port of `ichiran/dict:add-reading` (`dict-errata.lisp:35`).
//!
//! Inserts `reading` into the entry's kana or kanji table at the next
//! `ord` and bumps the entry's `n_kana` / `n_kanji`; returns the
//! (possibly updated) entry.

use super::entry_dao::Entry;
use super::kani_reading_table::KaniReadingTable;
use crate::characters::char_class_type::CharClass;
use crate::characters::test_word::test_word;
use crate::conn::kani_context::KaniranContext;

pub async fn add_reading(
    ctx: &KaniranContext,
    seq: i32,
    reading: &str,
    common: Option<i32>,
    conjugate_p: bool,
    table: Option<KaniReadingTable>,
) -> Result<Entry, sqlx::Error> {
    let is_kana = test_word(reading, CharClass::Kana);
    let table = table.unwrap_or(if is_kana {
        KaniReadingTable::Kana
    } else {
        KaniReadingTable::Kanji
    });
    let mut entry: Entry =
        sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
            .bind(seq)
            .fetch_one(&ctx.pool)
            .await?;
    let tname = table.table_name();
    let existing: Option<i32> = sqlx::query_scalar(&format!(
        "SELECT id FROM {} WHERE seq = $1 AND text = $2 LIMIT 1",
        tname
    ))
    .bind(seq)
    .bind(reading)
    .fetch_optional(&ctx.pool)
    .await?;
    if existing.is_none() {
        let maxord: Option<i32> = sqlx::query_scalar(&format!(
            "SELECT MAX(ord) FROM {} WHERE seq = $1",
            tname
        ))
        .bind(seq)
        .fetch_one(&ctx.pool)
        .await?;
        let ord = match maxord {
            None => 0,
            Some(m) => m + 1,
        };
        match table {
            KaniReadingTable::Kana => {
                // kana-text initforms (dict.lisp:128): common-tags "",
                // nokanji nil, best-kanji :null.
                sqlx::query(
                    "INSERT INTO kana_text \
                     (seq, text, ord, common, common_tags, conjugate_p, nokanji, best_kanji) \
                     VALUES ($1, $2, $3, $4, '', $5, FALSE, NULL)",
                )
                .bind(seq)
                .bind(reading)
                .bind(ord)
                .bind(common)
                .bind(conjugate_p)
                .execute(&ctx.pool)
                .await?;
                entry.n_kana += 1;
            }
            KaniReadingTable::Kanji => {
                // kanji-text initforms (dict.lisp:86): common-tags "",
                // nokanji nil, best-kana :null.
                sqlx::query(
                    "INSERT INTO kanji_text \
                     (seq, text, ord, common, common_tags, conjugate_p, nokanji, best_kana) \
                     VALUES ($1, $2, $3, $4, '', $5, FALSE, NULL)",
                )
                .bind(seq)
                .bind(reading)
                .bind(ord)
                .bind(common)
                .bind(conjugate_p)
                .execute(&ctx.pool)
                .await?;
                entry.n_kanji += 1;
            }
        }
        sqlx::query(
            "UPDATE entry SET content = $2, root_p = $3, n_kanji = $4, \
             n_kana = $5, primary_nokanji = $6 WHERE seq = $1",
        )
        .bind(entry.seq)
        .bind(&entry.content)
        .bind(entry.root_p)
        .bind(entry.n_kanji)
        .bind(entry.n_kana)
        .bind(entry.primary_nokanji)
        .execute(&ctx.pool)
        .await?;
    }
    Ok(entry)
}
