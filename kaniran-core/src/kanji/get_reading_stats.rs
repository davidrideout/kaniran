//! Port of `ichiran/kanji:get-reading-stats` (`kanji.lisp:399`).
//!
//! For a `(kanji, reading, type)` match, returns the
//! `(reading.stat_common, kanji.stat_common, perc, kanji.grade)`
//! tuple, or `None` when no row matches.

use super::calculate_perc::calculate_perc;
use crate::conn::kani_context::KaniranContext;

pub async fn get_reading_stats(
    ctx: &KaniranContext,
    kanji: &str,
    reading: &str,
    r#type: &str,
) -> Result<Option<(i32, i32, String, Option<i32>)>, sqlx::Error> {
    // kanji.lisp:401 ((:select 'r.stat-common 'k.stat-common 'k.grade ... :row))
    let row = sqlx::query_as::<_, (i32, i32, Option<i32>)>(
        "SELECT r.stat_common, k.stat_common, k.grade FROM kanji k, reading r \
         WHERE k.id = r.kanji_id AND k.text = $1 AND r.text = $2 AND r.type = $3",
    )
    .bind(kanji)
    .bind(reading)
    .bind(r#type)
    .fetch_all(&ctx.pool)
    .await?
    .into_iter()
    .next();
    Ok(row.map(|(sample, total, grade)| {
        (sample, total, calculate_perc(sample, total), grade)
    }))
}
