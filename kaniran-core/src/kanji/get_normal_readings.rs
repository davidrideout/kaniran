//! Port of `ichiran/kanji:get-normal-readings` (`kanji.lisp:231`).
//!
//! Looks up the kun/on readings of `char` in the `reading` table
//! (filtering out `ja_na` named-reading rows via
//! [`super::get_readings_cache`]), expands each into the geminate /
//! rendaku variants via [`super::get_reading_alternatives`], then
//! deduplicates by reading text keeping the first occurrence in
//! traversal order.
//!
//! Diverges from the upstream lambda list `(char &key rendaku)` by:
//!
//! - taking `&KaniranContext` for the database handle, replacing the
//!   upstream dynamic `*connection*` per
//!   [`crate::conn::kani_context`];
//! - taking `rendaku` as a plain `bool` per CONVENTIONS §4.4. The
//!   keyword is binary (absent ↔ off, `t` ↔ on);
//! - returning [`Vec<KanjiReading>`] in place of the heterogeneous
//!   3-/4-element cons lists upstream emits — the conversion is
//!   lossless and is shared with [`super::make_rmap`] /
//!   [`super::match_readings`].
//!
//! Upstream's `:from-end t` in `remove-duplicates` empirically keeps
//! the **first** occurrence in original-order (verified via REPL):
//! `(remove-duplicates '((1 a) (2 b) (1 c) (3 d)) :test 'equal :key
//! 'car :from-end t)` → `((1 A) (2 B) (3 D))`. The Rust port
//! mirrors that — the rendaku-induced "び" of 日 is shadowed by the
//! original `("び" "ja_kun" NIL)` entry.

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
