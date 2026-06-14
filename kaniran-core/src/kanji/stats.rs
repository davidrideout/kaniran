use crate::conn::kani_backend::KaniBackend;
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
pub fn kanji_word_stats(
    ctx: &KaniranContext,
    char: &str,
) -> Result<(Vec<((String, String), i32)>, i32, usize), crate::conn::KaniDbError> {
    let str = char;
    let words = get_kanji_words(ctx, str)?;
    let mut r_stat: HashMap<(String, String), i32> = HashMap::new();
    let mut irregular: i32 = 0;

    // kanji.lisp:321-329 (loop for (seq k r common) in words …)
    for (_seq, k, r, _common) in &words {
        let mr = match_readings(ctx, k, r)?;
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
pub fn get_reading_stats(
    ctx: &KaniranContext,
    kanji: &str,
    reading: &str,
    r#type: &str,
) -> Result<Option<(i32, i32, String, Option<i32>)>, crate::conn::KaniDbError> {
    // kanji.lisp:401 ((:select 'r.stat-common 'k.stat-common 'k.grade ... :row))
    let row = ctx
        .store
        .reading_stats_rows(kanji, reading, r#type)
        ?
        .into_iter()
        .next();
    Ok(row.map(|(sample, total, grade)| (sample, total, calculate_perc(sample, total), grade)))
}

#[cfg(test)]
mod tests;
