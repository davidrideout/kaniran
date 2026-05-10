//! Port of `ichiran/kanji:get-reading-stats` (`kanji.lisp:399`).
//!
//! Joins `kanji` and `reading` on `kanji.id = reading.kanji_id`,
//! filters by `kanji.text = $1 AND reading.text = $2 AND
//! reading.type = $3`, and returns the matched row's
//! `(reading.stat_common, kanji.stat_common, perc, kanji.grade)`
//! tuple — where `perc` is rendered via [`super::calculate_perc`].
//! Returns `None` when no row matches.
//!
//! Diverges from the upstream lambda list `(kanji reading type)` only
//! by taking `&KaniranContext` for the database handle, replacing the
//! upstream dynamic `*connection*` per [`crate::conn::kani_context`].
//! `grade` stays `Option<i32>` because the column is nullable and
//! upstream surfaces `:null` to the caller.

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
