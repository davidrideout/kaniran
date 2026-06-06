//! Port of `ichiran/kanji:get-normal-readings` (`kanji.lisp:231`).
//!
//! Looks up the kun/on readings of `char` (excluding `ja_na`
//! named-reading rows), expands each into geminate / rendaku variants,
//! then deduplicates by reading text keeping the first occurrence.

use std::collections::HashSet;

use super::get_reading_alternatives::get_reading_alternatives;
use super::get_readings_cache::get_readings_cache;
use super::kani_kanji_reading::KanjiReading;
use crate::conn::kani_context::KaniranContext;

pub async fn get_normal_readings(
    ctx: &KaniranContext,
    char: char,
    rendaku: bool,
) -> Result<Vec<KanjiReading>, sqlx::Error> {
    let str: String = char.into();
    let typeset = vec!["ja_na".to_string()];
    let readings = get_readings_cache(ctx, &str, &typeset).await?;

    let mut main_readings: Vec<KanjiReading> = Vec::new();
    let mut alt_readings: Vec<KanjiReading> = Vec::new();
    for (reading, r#type) in &readings {
        let alternatives = get_reading_alternatives(reading, r#type, rendaku);
        // kanji.lisp:235 (loop ... for (main . rest) = ...)
        let mut iter = alternatives.into_iter();
        if let Some(main) = iter.next() {
            main_readings.push(main.into());
        }
        for alt in iter {
            alt_readings.push(alt.into());
        }
    }

    let mut combined: Vec<KanjiReading> = main_readings;
    combined.extend(alt_readings);

    // kanji.lisp:239 (remove-duplicates ... :test 'equal :key 'car :from-end t)
    // — keep the first occurrence in original order (verified empirically).
    let mut seen: HashSet<String> = HashSet::new();
    let mut deduped: Vec<KanjiReading> = Vec::with_capacity(combined.len());
    for entry in combined {
        if seen.insert(entry.reading.clone()) {
            deduped.push(entry);
        }
    }
    Ok(deduped)
}
