//! Port of `ichiran/dict:replace-reading` (`dict-errata.lisp:49`).
//!
//! Renames every row of the entry's kana or kanji table from
//! `reading_from` to `reading_to`, then calls [`reset_readings`] iff
//! at least one row was updated.

use super::reset_readings::reset_readings;
use crate::characters::char_class::CharClass;
use crate::characters::char_class::test_word;
use crate::conn::kani_context::KaniranContext;

pub async fn replace_reading(
    ctx: &KaniranContext,
    seq: i32,
    reading_from: &str,
    reading_to: &str,
) -> Result<(), sqlx::Error> {
    let is_kana = test_word(reading_from, CharClass::Kana);
    let tname = if is_kana { "kana_text" } else { "kanji_text" };
    let updated = sqlx::query(&format!(
        "UPDATE {} SET text = $1 WHERE seq = $2 AND text = $3",
        tname
    ))
    .bind(reading_to)
    .bind(seq)
    .bind(reading_from)
    .execute(&ctx.pool)
    .await?
    .rows_affected();
    if updated > 0 {
        reset_readings(ctx, &[seq]).await?;
    }
    Ok(())
}
