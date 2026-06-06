//! Port of `ichiran/kanji:match-readings` (`kanji.lisp:292`).
//!
//! Pairs each character of `str` with the matched reading entry
//! produced by [`super::match_readings_star_`]: kanji-positions
//! become `MatchedSegment::Kanji { kanji, reading }` (one-character
//! kanji string + the matched reading variant), and runs of
//! consecutive non-kanji characters collapse into one
//! `MatchedSegment::NonKanji(string)` item. Returns `None` when the
//! reading does not match the character map at all.

use super::kani_kanji_reading::KanjiReading;
use super::make_rmap::make_rmap;
use super::match_readings_star_::{match_readings_star, MatchItem, MatchResult};
use crate::conn::kani_context::KaniranContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchedSegment {
    NonKanji(String),
    Kanji { kanji: String, reading: KanjiReading },
}

pub async fn match_readings(
    ctx: &KaniranContext,
    str: &str,
    reading: &str,
) -> Result<Option<Vec<MatchedSegment>>, sqlx::Error> {
    let rmap = make_rmap(ctx, str).await?;
    let match_result = match_readings_star(&rmap, reading);
    let items = match match_result {
        MatchResult::None => return Ok(None),
        MatchResult::Some { items, .. } => items,
    };

    // kanji.lisp:296-306 (loop with charbag and result for m in match for c across str ...)
    let mut charbag: Vec<char> = Vec::new();
    let mut result: Vec<MatchedSegment> = Vec::new();
    for (m, c) in items.iter().zip(str.chars()) {
        match m {
            MatchItem::Reading(r) => {
                if !charbag.is_empty() {
                    result.push(MatchedSegment::NonKanji(charbag.iter().collect()));
                    charbag.clear();
                }
                let mut kanji = String::new();
                kanji.push(c);
                result.push(MatchedSegment::Kanji {
                    kanji,
                    reading: r.clone(),
                });
            }
            MatchItem::Char(_) => {
                charbag.push(c);
            }
        }
    }
    if !charbag.is_empty() {
        result.push(MatchedSegment::NonKanji(charbag.iter().collect()));
    }
    Ok(Some(result))
}
