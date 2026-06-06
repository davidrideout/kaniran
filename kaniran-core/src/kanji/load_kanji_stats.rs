//! Port of `ichiran/kanji:load-kanji-stats` (`kanji.lisp:332`).
//!
//! Recomputes the per-kanji and per-reading word-stat counters. Zeroes
//! every `kanji.stat_common` / `kanji.stat_irregular` /
//! `reading.stat_common` first, then iterates every kanji with
//! `grade <= 8`, runs
//! [`super::kanji_word_stats::kanji_word_stats`] against the kanji
//! literal, writes the `(total, irregular)` pair back onto the kanji
//! row, and writes each `((rtext, rtype), count)` tally back onto the
//! matching `reading` row (matched by `(text, type, kanji_id)`).

use super::kanji_dao::Kanji;
use super::kanji_word_stats::kanji_word_stats;
use super::reading_dao::Reading;
use crate::conn::kani_context::KaniranContext;

pub async fn load_kanji_stats(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    // kanji.lisp:334 (query (:update 'kanji :set 'stat-common 0 'stat-irregular 0))
    sqlx::query("UPDATE kanji SET stat_common = 0, stat_irregular = 0")
        .execute(&ctx.pool)
        .await?;
    // kanji.lisp:335 (query (:update 'reading :set 'stat-common 0))
    sqlx::query("UPDATE reading SET stat_common = 0")
        .execute(&ctx.pool)
        .await?;
    // kanji.lisp:336 (select-dao 'kanji (:<= 'grade 8))
    let kanji_rows: Vec<Kanji> =
        sqlx::query_as("SELECT * FROM kanji WHERE grade <= 8")
            .fetch_all(&ctx.pool)
            .await?;
    // kanji.lisp:336-347 (loop for kanji in … …)
    // `cnt` starts at 1 and steps after each body (CL loop `for cnt from 1`
    // post-increments), so the `finally` clause sees `cnt = N+1`.
    let mut cnt: i32 = 1;
    for kanji in &kanji_rows {
        // kanji.lisp:338 ((reading-stats irregular total) = (multiple-value-list (kanji-word-stats (text kanji))))
        let (reading_stats, irregular, total) = kanji_word_stats(ctx, &kanji.text).await?;
        // kanji.lisp:339 (readings = (select-dao 'reading (:= 'kanji-id (id kanji))))
        let readings: Vec<Reading> =
            sqlx::query_as("SELECT * FROM reading WHERE kanji_id = $1")
                .bind(kanji.id)
                .fetch_all(&ctx.pool)
                .await?;
        // kanji.lisp:340-342 (setf (stat-common kanji) total (stat-irregular kanji) irregular) (update-dao kanji)
        sqlx::query(
            "UPDATE kanji SET stat_common = $1, stat_irregular = $2 WHERE id = $3",
        )
        .bind(total as i32)
        .bind(irregular)
        .bind(kanji.id)
        .execute(&ctx.pool)
        .await?;
        // kanji.lisp:343-345 (loop for ((rtext rtype) . rcount) in reading-stats …)
        for ((rtext, rtype), rcount) in &reading_stats {
            // kanji.lisp:344 (find-if (lambda (r) (and (equal (text r) rtext) (equal (reading-type r) rtype))) readings)
            let reading = readings
                .iter()
                .find(|r| r.text == *rtext && r.reading_type == *rtype);
            if let Some(reading) = reading {
                sqlx::query("UPDATE reading SET stat_common = $1 WHERE id = $2")
                    .bind(*rcount)
                    .bind(reading.id)
                    .execute(&ctx.pool)
                    .await?;
            }
        }
        // kanji.lisp:346 (if (zerop (mod cnt 100)) do (format t "~a kanji processed~%" cnt))
        if cnt % 100 == 0 {
            println!("{cnt} kanji processed");
        }
        cnt += 1;
    }
    // kanji.lisp:347 (finally (query "ANALYZE") (format t "~a kanji total~%" cnt))
    sqlx::query("ANALYZE").execute(&ctx.pool).await?;
    println!("{cnt} kanji total");
    Ok(())
}
