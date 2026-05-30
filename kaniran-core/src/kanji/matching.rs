//! Reading-to-string alignment. From `kanji.lisp:241-302`.

use crate::characters::char_classes::KANJI_REGEX;
use crate::conn::kani_context::KaniranContext;

use super::readings::{get_normal_readings, KanjiReading, ReadingTag};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RmapEntry {
    NonKanji(char),
    Readings(Vec<KanjiReading>),
}

/// `make-rmap` (`kanji.lisp:273`). One entry per character of `str`:
/// `NonKanji(c)` passthrough or `Readings(...)` with the candidate
/// readings. Normal kanji use [`get_normal_readings`]; the abbreviation
/// marks `ヶ` and `〆` carry per-character literals; the iteration mark
/// `々` propagates the previous kanji's readings with `rendaku=t`.
pub async fn make_rmap(
    ctx: &KaniranContext,
    str: &str,
) -> Result<Vec<RmapEntry>, sqlx::Error> {
    let chars: Vec<char> = str.chars().collect();
    let mut out: Vec<RmapEntry> = Vec::with_capacity(chars.len());
    let mut prev_kanji: Option<char> = None;

    for (start, &c) in chars.iter().enumerate() {
        if is_kanji(c) {
            let entry = match c {
                '々' => {
                    // kanji.lisp:279-281 (eql char #\々) — propagate prev with rendaku
                    let readings = match prev_kanji {
                        Some(prev) => get_normal_readings(ctx, prev, true).await?,
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
                    RmapEntry::Readings(get_normal_readings(ctx, c, rendaku).await?)
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

/// `*kanji-regex*` matches one of `々ヶ〆` or `[一-龯]`; a direct char
/// compare is equivalent and faster for one-codepoint membership.
fn is_kanji(c: char) -> bool {
    debug_assert_eq!(KANJI_REGEX, "[々ヶ〆一-龯]");
    matches!(c, '々' | 'ヶ' | '〆' | '\u{4e00}'..='\u{9faf}')
}

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

/// `match-readings*` (`kanji.lisp:241`). Recursively match a `reading`
/// against an `rmap`. For each kanji position: either align a candidate
/// against a prefix of the remaining `reading`, or emit an `"irr"`
/// fallback covering the consumed prefix verbatim (score penalised by
/// the prefix length so candidates always beat same-span irrs). For
/// each non-kanji position the next `reading` char must match exactly.
/// Best-score wins; ties resolve to the largest `end` to mirror the
/// upstream loop's strict `>` against `max-score` over a reverse-push
/// iteration order.
///
/// Upstream `:start` keyword dropped (always 0 in-tree); recursion
/// threads the offset through an internal helper. The upstream
/// `(values match score) | :none` triple-shape return collapses to
/// `MatchResult::None` / `::Some { items, score }`.
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
                        // kanji.lisp:259 (push (... "irr") match) (- score (- end start))
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

            // kanji.lisp:260-266: strict `>` walks the push-stack tail-first, so
            // for ties the largest-end push wins. `max_by_key` returns the last
            // equal max — push order is smallest-end first, so the last equal
            // max is the largest-end entry, mirroring the Lisp.
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchedSegment {
    NonKanji(String),
    Kanji { kanji: String, reading: KanjiReading },
}

/// `match-readings` (`kanji.lisp:292`). Pair each character of `str`
/// with [`match_readings_star`]'s output: kanji positions become
/// `Kanji { kanji, reading }`, runs of consecutive non-kanji collapse
/// into one `NonKanji(string)`. `None` when the reading cannot align.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(text: &str, ty: &str) -> KanjiReading {
        KanjiReading {
            reading: text.to_string(),
            r#type: ty.to_string(),
            tag: None,
            gem: None,
        }
    }

    #[test]
    fn empty_rmap_with_exhausted_reading_returns_zero_score() {
        match match_readings_star(&[], "") {
            MatchResult::Some { items, score } => {
                assert!(items.is_empty());
                assert_eq!(score, 0);
            }
            MatchResult::None => panic!("expected Some, got None"),
        }
    }

    #[test]
    fn empty_rmap_with_remaining_reading_returns_none() {
        assert_eq!(match_readings_star(&[], "abc"), MatchResult::None);
    }

    #[test]
    fn nonempty_rmap_with_exhausted_reading_returns_none() {
        let rmap = vec![RmapEntry::Readings(vec![reading("a", "x")])];
        assert_eq!(match_readings_star(&rmap, ""), MatchResult::None);
    }

    #[test]
    fn nonkanji_passthrough_matches_exact_char() {
        let rmap = vec![RmapEntry::NonKanji('っ')];
        let result = match_readings_star(&rmap, "っ");
        assert_eq!(
            result,
            MatchResult::Some {
                items: vec![MatchItem::Char('っ')],
                score: 0,
            }
        );
    }

    #[test]
    fn nonkanji_mismatch_returns_none() {
        let rmap = vec![RmapEntry::NonKanji('っ')];
        assert_eq!(match_readings_star(&rmap, "あ"), MatchResult::None);
    }

    #[test]
    fn matching_reading_keeps_score_zero() {
        let rmap = vec![RmapEntry::Readings(vec![
            reading("ひ", "ja_kun"),
            reading("にち", "ja_on"),
        ])];
        match match_readings_star(&rmap, "ひ") {
            MatchResult::Some { items, score } => {
                assert_eq!(score, 0);
                assert_eq!(items.len(), 1);
                match &items[0] {
                    MatchItem::Reading(r) => {
                        assert_eq!(r.reading, "ひ");
                        assert_eq!(r.r#type, "ja_kun");
                    }
                    _ => panic!("expected Reading, got Char"),
                }
            }
            MatchResult::None => panic!("expected Some, got None"),
        }
    }

    #[test]
    fn unmatched_reading_falls_back_to_irr_with_negative_score() {
        let rmap = vec![RmapEntry::Readings(vec![reading("ひ", "ja_kun")])];
        match match_readings_star(&rmap, "ぜ") {
            MatchResult::Some { items, score } => {
                assert_eq!(score, -1);
                assert_eq!(items.len(), 1);
                match &items[0] {
                    MatchItem::Reading(r) => {
                        assert_eq!(r.reading, "ぜ");
                        assert_eq!(r.r#type, "irr");
                    }
                    _ => panic!("expected Reading, got Char"),
                }
            }
            MatchResult::None => panic!("expected Some, got None"),
        }
    }

    /// Per upstream tie-break (largest end wins), the [0..2]-span result
    /// should be selected.
    #[test]
    fn ties_break_to_largest_end() {
        let rmap = vec![
            RmapEntry::Readings(vec![reading("あい", "x")]),
            RmapEntry::Readings(vec![reading("う", "y")]),
        ];
        match match_readings_star(&rmap, "あいう") {
            MatchResult::Some { items, score } => {
                assert_eq!(score, 0);
                assert_eq!(items.len(), 2);
                if let MatchItem::Reading(r0) = &items[0] {
                    assert_eq!(r0.reading, "あい");
                } else {
                    panic!("expected Reading at 0");
                }
                if let MatchItem::Reading(r1) = &items[1] {
                    assert_eq!(r1.reading, "う");
                } else {
                    panic!("expected Reading at 1");
                }
            }
            MatchResult::None => panic!("expected Some, got None"),
        }
    }
}
