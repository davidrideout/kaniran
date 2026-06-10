use crate::conn::kani_backend::KaniBackend;
use super::dao::{Kanji, Reading};
use super::matching::{match_readings, MatchedSegment};
use super::readings::{get_original_reading, ReadingTag};
use crate::conn::kani_context::KaniranContext;
use crate::dict::word_info_str::get_kanji_words;
use std::collections::HashMap;

/// Port of `ichiran/kanji:kanji-word-stats` (`kanji.lisp:316`).
///
/// For every kanji-bearing common word that contains `char`,
/// aligns the kanji writing with its kana reading via
/// [`super::match_readings`] and tallies how many words ascribe each
/// `(reading, type)` pair to `char`. Words whose alignment falls
/// through to an `"irr"` reading (or fails to align altogether) are
/// counted into the `irregular` tally. Returns the tally alist,
/// the irregular count, and the total word count.
pub async fn kanji_word_stats(
    ctx: &KaniranContext,
    char: &str,
) -> Result<(Vec<((String, String), i32)>, i32, usize), sqlx::Error> {
    let str = char;
    let words = get_kanji_words(ctx, str).await?;
    let mut r_stat: HashMap<(String, String), i32> = HashMap::new();
    let mut irregular: i32 = 0;

    // kanji.lisp:321-329 (loop for (seq k r common) in words …)
    for (_seq, k, r, _common) in &words {
        let mr = match_readings(ctx, k, r).await?;
        // kanji.lisp:322 (assoc str (remove-if-not 'listp (match-readings k r)) :test 'equal)
        let reading = mr.as_deref().and_then(|segs| {
            segs.iter().find(|seg| match seg {
                MatchedSegment::Kanji { kanji, .. } => kanji == str,
                MatchedSegment::NonKanji(_) => false,
            })
        });
        match reading {
            Some(MatchedSegment::Kanji { reading: kr, .. }) => {
                // kanji.lisp:324-328 (destructuring-bind (rtext rtype &rest options) (cdr reading) …)
                let rtext = &kr.reading;
                let rtype = &kr.r#type;
                if rtype == "irr" {
                    irregular += 1;
                } else {
                    let rendaku = matches!(kr.tag, Some(ReadingTag::Rendaku));
                    let geminated = kr.gem.as_deref();
                    let key_text = get_original_reading(rtext, rendaku, geminated);
                    let key = (key_text, rtype.clone());
                    *r_stat.entry(key).or_insert(0) += 1;
                }
            }
            _ => {
                // kanji.lisp:329 (else do (incf irregular))
                irregular += 1;
            }
        }
    }
    // kanji.lisp:330 (values (alexandria:hash-table-alist r-stat) irregular (length words))
    let stats: Vec<((String, String), i32)> = r_stat.into_iter().collect();
    Ok((stats, irregular, words.len()))
}

/// Port of `ichiran/kanji:load-kanji-stats` (`kanji.lisp:332`).
///
/// Recomputes the per-kanji and per-reading word-stat counters. Zeroes
/// every `kanji.stat_common` / `kanji.stat_irregular` /
/// `reading.stat_common` first, then iterates every kanji with
/// `grade <= 8`, runs
/// [`super::stats::kanji_word_stats`] against the kanji
/// literal, writes the `(total, irregular)` pair back onto the kanji
/// row, and writes each `((rtext, rtype), count)` tally back onto the
/// matching `reading` row (matched by `(text, type, kanji_id)`).
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
    let kanji_rows: Vec<Kanji> = sqlx::query_as("SELECT * FROM kanji WHERE grade <= 8")
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
        let readings: Vec<Reading> = sqlx::query_as("SELECT * FROM reading WHERE kanji_id = $1")
            .bind(kanji.id)
            .fetch_all(&ctx.pool)
            .await?;
        // kanji.lisp:340-342 (setf (stat-common kanji) total (stat-irregular kanji) irregular) (update-dao kanji)
        sqlx::query("UPDATE kanji SET stat_common = $1, stat_irregular = $2 WHERE id = $3")
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

/// Port of `ichiran/kanji:calculate-perc` (`kanji.lisp:349`).
///
/// Renders `sample / total` as a percentage string with two fractional
/// digits and a trailing `%`, or the literal `"--.--%"` when `total`
/// is zero.
pub fn calculate_perc(sample: i32, total: i32) -> String {
    if total == 0 {
        "--.--%".to_string()
    } else {
        format!("{:.2}%", 100.0 * sample as f64 / total as f64)
    }
}

/// Port of `ichiran/kanji:get-reading-stats` (`kanji.lisp:399`).
///
/// For a `(kanji, reading, type)` match, returns the
/// `(reading.stat_common, kanji.stat_common, perc, kanji.grade)`
/// tuple, or `None` when no row matches.
pub async fn get_reading_stats(
    ctx: &KaniranContext,
    kanji: &str,
    reading: &str,
    r#type: &str,
) -> Result<Option<(i32, i32, String, Option<i32>)>, sqlx::Error> {
    // kanji.lisp:401 ((:select 'r.stat-common 'k.stat-common 'k.grade ... :row))
    let row = ctx
        .store
        .reading_stats_rows(kanji, reading, r#type)
        .await?
        .into_iter()
        .next();
    Ok(row.map(|(sample, total, grade)| (sample, total, calculate_perc(sample, total), grade)))
}

#[cfg(test)]
mod tests;
