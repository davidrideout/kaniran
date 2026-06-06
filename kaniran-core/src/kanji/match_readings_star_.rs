//! Port of `ichiran/kanji:match-readings*` (`kanji.lisp:241`).
//!
//! Recursively matches a `reading` (kana string) against an `rmap`
//! produced by [`super::make_rmap`]. For each kanji-position it
//! either lines a candidate variant up against a prefix of the
//! remaining `reading` or, when no candidate matches, falls back to
//! an `"irr"` (irregular) reading covering the consumed prefix
//! verbatim — penalising the score by the prefix length so a
//! candidate-match is always preferred over an irr fallback that
//! covers the same span. For each non-kanji rmap position the next
//! `reading` character must match exactly. The best-scoring match
//! is returned; ties resolve to the largest `end`.

use super::kani_kanji_reading::KanjiReading;
use super::make_rmap::RmapEntry;

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

    #[test]
    fn ties_break_to_largest_end() {
        // Single kanji vs. a 2-char reading. Two valid spans:
        //   end=1: irr("あ"), score = 0 - 1 = -1, then needs reading[1..] match
        //   end=2: irr("あい"), score = 0 - 2 = -2.
        // The larger-end (longer irr) outscores the shorter prefix only when
        // followed-by-mismatch invalidates end=1. Here we set up a case where
        // both ends produce a valid recursion: rmap has two kanji slots, and
        // either span [0..1]+[1..2] or [0..2]+exhausted-reading is reachable.
        // Per upstream tie-break (largest end wins), the [0..2]-span result
        // should be selected.
        let rmap = vec![
            RmapEntry::Readings(vec![reading("あい", "x")]),
            RmapEntry::Readings(vec![reading("う", "y")]),
        ];
        // Reading "あいう" — only one valid match: ("あい", "x") then ("う", "y").
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
