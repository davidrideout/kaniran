use super::kani_kanji_reading::KanjiReading;
use super::readings::{get_normal_readings, ReadingTag};
use crate::characters::constants::KANJI_REGEX;
use crate::conn::kani_context::KaniranContext;

/// Port of `ichiran/kanji:match-readings*` (`kanji.lisp:241`).
///
/// Recursively matches a `reading` (kana string) against an `rmap`
/// produced by [`super::make_rmap`]. For each kanji-position it
/// either lines a candidate variant up against a prefix of the
/// remaining `reading` or, when no candidate matches, falls back to
/// an `"irr"` (irregular) reading covering the consumed prefix
/// verbatim — penalising the score by the prefix length so a
/// candidate-match is always preferred over an irr fallback that
/// covers the same span. For each non-kanji rmap position the next
/// `reading` character must match exactly. The best-scoring match
/// is returned; ties resolve to the largest `end`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchItem {
    Char(char),
    Reading(KanjiReading),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchResult {
    None,
    Some { items: Vec<MatchItem>, score: i64 },
}

pub fn match_readings_star(rmap: &[RmapEntry], reading: &str) -> MatchResult {
    let chars: Vec<char> = reading.chars().collect();
    inner(rmap, &chars, 0)
}

fn inner(rmap: &[RmapEntry], reading: &[char], start: usize) -> MatchResult {
    // kanji.lisp:242-246 (unless rmap ...)
    if rmap.is_empty() {
        return if start >= reading.len() {
            MatchResult::Some {
                items: Vec::new(),
                score: 0,
            }
        } else {
            MatchResult::None
        };
    }
    // kanji.lisp:247-248 (when (>= start (length reading)) :none)
    if start >= reading.len() {
        return MatchResult::None;
    }

    match &rmap[0] {
        // kanji.lisp:252-266 ((listp item) ...)
        RmapEntry::Readings(item) => {
            let mut matches: Vec<(Vec<MatchItem>, i64)> = Vec::new();
            for end in (start + 1)..=reading.len() {
                let result = inner(&rmap[1..], reading, end);
                let (rest_items, rest_score) = match result {
                    MatchResult::None => continue,
                    MatchResult::Some { items, score } => (items, score),
                };
                let chunk: &[char] = &reading[start..end];

                // kanji.lisp:256-258 (loop for r in item ... unless mismatch ...)
                let matched_variant = item
                    .iter()
                    .find(|r| r.reading.chars().eq(chunk.iter().copied()));

                let (head, score) = match matched_variant {
                    Some(r) => (MatchItem::Reading(r.clone()), rest_score),
                    None => {
                        // kanji.lisp:259 (push (cons (cons (list (subseq reading start end) "irr") match) (- score (- end start))) ...)
                        let chunk_str: String = chunk.iter().collect();
                        (
                            MatchItem::Reading(KanjiReading {
                                reading: chunk_str,
                                r#type: "irr".to_string(),
                                tag: None,
                                gem: None,
                            }),
                            rest_score - (end - start) as i64,
                        )
                    }
                };

                let mut combined: Vec<MatchItem> = Vec::with_capacity(1 + rest_items.len());
                combined.push(head);
                combined.extend(rest_items);
                matches.push((combined, score));
            }

            // kanji.lisp:260-266 (if matches (loop ... best-match)) — strict `>`
            // walks the push-stack tail-first, so for ties the largest-end push
            // (last pushed) wins. `max_by_key` returns the last equal max — the
            // matches Vec is in push order (smallest-end first), so the last
            // equal max is the largest-end entry, mirroring the Lisp.
            match matches.into_iter().max_by_key(|(_, score)| *score) {
                Some((items, score)) => MatchResult::Some { items, score },
                None => MatchResult::None,
            }
        }
        // kanji.lisp:267-271 ((t (if (eql item (char reading start)) ...)))
        RmapEntry::NonKanji(c) => {
            if *c == reading[start] {
                match inner(&rmap[1..], reading, start + 1) {
                    MatchResult::None => MatchResult::None,
                    MatchResult::Some { items, score } => {
                        let mut combined: Vec<MatchItem> = Vec::with_capacity(1 + items.len());
                        combined.push(MatchItem::Char(*c));
                        combined.extend(items);
                        MatchResult::Some {
                            items: combined,
                            score,
                        }
                    }
                }
            } else {
                MatchResult::None
            }
        }
    }
}

/// Port of `ichiran/kanji:make-rmap` (`kanji.lisp:273`).
///
/// Walks `str` once and produces one [`RmapEntry`] per character:
/// `RmapEntry::NonKanji(c)` for non-kanji passthroughs, or
/// `RmapEntry::Readings(...)` listing the candidate readings for
/// each kanji-ish character. The candidate list comes from
/// [`super::get_normal_readings`] for normal kanji; from a
/// per-character literal for the abbreviation marks ヶ and 〆; and
/// propagates the previous kanji's readings (with `rendaku=t`) for
/// the iteration mark 々.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RmapEntry {
    NonKanji(char),
    Readings(Vec<KanjiReading>),
}

pub fn make_rmap(ctx: &KaniranContext, str: &str) -> Result<Vec<RmapEntry>, crate::conn::KaniDbError> {
    let chars: Vec<char> = str.chars().collect();
    let mut out: Vec<RmapEntry> = Vec::with_capacity(chars.len());
    let mut prev_kanji: Option<char> = None;

    for (start, &c) in chars.iter().enumerate() {
        if is_kanji(c) {
            let entry = match c {
                '々' => {
                    // kanji.lisp:279-281 (eql char #\々) — propagate prev with rendaku
                    let readings = match prev_kanji {
                        Some(prev) => get_normal_readings(ctx, prev, true)?,
                        None => Vec::new(),
                    };
                    prev_kanji = None;
                    RmapEntry::Readings(readings)
                }
                'ヶ' => {
                    // kanji.lisp:282-284 (eql char #\ヶ)
                    prev_kanji = None;
                    RmapEntry::Readings(vec![
                        KanjiReading {
                            reading: "か".to_string(),
                            r#type: "ja_on".to_string(),
                            tag: None,
                            gem: None,
                        },
                        KanjiReading {
                            reading: "が".to_string(),
                            r#type: "abbr".to_string(),
                            tag: None,
                            gem: None,
                        },
                    ])
                }
                '〆' => {
                    // kanji.lisp:285-287 (eql char #\〆) — prev-kanji becomes 締
                    prev_kanji = Some('締');
                    RmapEntry::Readings(vec![
                        KanjiReading {
                            reading: "しめ".to_string(),
                            r#type: "ja_kun".to_string(),
                            tag: None,
                            gem: None,
                        },
                        KanjiReading {
                            reading: "じめ".to_string(),
                            r#type: "ja_kun".to_string(),
                            tag: Some(ReadingTag::Rendaku),
                            gem: None,
                        },
                    ])
                }
                _ => {
                    // kanji.lisp:288-289 (t (setf prev-kanji char) ...)
                    prev_kanji = Some(c);
                    let rendaku = start > 0;
                    RmapEntry::Readings(get_normal_readings(ctx, c, rendaku)?)
                }
            };
            out.push(entry);
        } else {
            // kanji.lisp:290 (else collect char)
            out.push(RmapEntry::NonKanji(c));
        }
    }
    Ok(out)
}

/// `*kanji-regex*` matches a single character against
/// `[々ヶ〆一-龯]`. The regex engine is overkill for one-codepoint
/// membership; a direct char comparison is equivalent.
fn is_kanji(c: char) -> bool {
    debug_assert_eq!(KANJI_REGEX, "[々ヶ〆一-龯]");
    matches!(c, '々' | 'ヶ' | '〆' | '\u{4e00}'..='\u{9faf}')
}

/// Port of `ichiran/kanji:match-readings` (`kanji.lisp:292`).
///
/// Pairs each character of `str` with the matched reading entry
/// produced by [`super::match_readings_star_`]: kanji-positions
/// become `MatchedSegment::Kanji { kanji, reading }` (one-character
/// kanji string + the matched reading variant), and runs of
/// consecutive non-kanji characters collapse into one
/// `MatchedSegment::NonKanji(string)` item. Returns `None` when the
/// reading does not match the character map at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchedSegment {
    NonKanji(String),
    Kanji {
        kanji: String,
        reading: KanjiReading,
    },
}

pub fn match_readings(
    ctx: &KaniranContext,
    str: &str,
    reading: &str,
) -> Result<Option<Vec<MatchedSegment>>, crate::conn::KaniDbError> {
    let rmap = make_rmap(ctx, str)?;
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

#[cfg(test)]
mod tests;
