//! View-models for the reader page.
//!
//! Everything the templates need is precomputed here from the v2 document
//! plus the difficulty tables, so the templates stay flat loops and field
//! access: sense resolution (entry id → glosses), furigana fallbacks,
//! compound grouping, and the sidebar histograms.

use std::collections::BTreeMap;

use serde::Serialize;

use kaniran_core::serializers::{V2Document, V2Entry, V2Sense, V2Token};

use crate::levels::Levels;

/// Cap on senses shown in a word tooltip — polysemous words like する carry
/// dozens. The overflow renders as "+N more senses".
const MAX_TOOLTIP_SENSES: usize = 5;

/// Template context for the reader page.
#[derive(Debug, Serialize)]
pub struct ReaderView {
    /// The analyzed input, echoed back into the textarea.
    pub text: String,
    /// Words + kept punctuation, grouped for display.
    pub chunks: Vec<Chunk>,
    /// Dictionary words in the best path.
    pub word_count: usize,
    /// Kanji occurrences across those words.
    pub kanji_count: usize,
    /// Word-JLPT histogram (always all six rows, N5 → unrated).
    pub jlpt_rows: Vec<BarRow>,
    /// Kanji histogram by Joyo grade; empty when the text has no kanji.
    pub grade_rows: Vec<BarRow>,
    /// Kanji histogram by WaniKani level bucket; empty likewise.
    pub wk_rows: Vec<BarRow>,
    /// Server-side segmentation time, preformatted to one decimal for the
    /// "N words parsed in X ms" stats line.
    pub parse_ms: String,
}

/// A run of tokens rendered as one visual unit: either a single token, or
/// the members of one compound (e.g. 勉強しています → 勉強 / して / います)
/// grouped so they read as one word.
#[derive(Debug, Serialize)]
pub struct Chunk {
    pub compound: bool,
    pub tokens: Vec<TokenView>,
}

/// One rendered token. `gap` marks non-word spans (punctuation, whitespace,
/// latin) that render verbatim; everything else is a dictionary word with
/// furigana, an under-word gloss, and a pre-rendered tooltip body.
#[derive(Debug, Serialize)]
pub struct TokenView {
    pub gap: bool,
    /// Verbatim input slice.
    pub text: String,
    /// Kana reading, omitted when it adds nothing (equal to `text`).
    pub reading: Option<String>,
    pub romanization: String,
    /// Ruby cells in surface order; `reading` present only over kanji runs.
    pub furigana: Vec<FuriCell>,
    /// First sense's gloss line, shown under the word.
    pub gloss: Option<String>,
    /// Tooltip definition groups, capped at [`MAX_TOOLTIP_SENSES`] lines total.
    pub sense_groups: Vec<SenseGroupView>,
    /// How many senses the cap hid.
    pub extra_senses: usize,
    /// Conjugation analyses (root + derivation description), for the tooltip.
    pub conjugation: Vec<ConjView>,
    /// Suffix-grammar description when this token is a compound's suffix
    /// member (e.g. て-form auxiliary).
    pub suffix: Option<String>,
    /// True when any sense is a particle — drives the particle styling and
    /// the "particle glosses" toggle.
    pub is_particle: bool,
    /// JMdict sequence number of the resolved entry.
    pub seq: Option<i32>,
    /// Word-level JLPT N-number (5 = N5 … 1 = N1), if listed.
    pub jlpt: Option<u8>,
}

/// One ruby cell: `{text: 暖, reading: あたた}` renders
/// `<ruby>暖<rt>あたた</rt></ruby>`; without a reading, bare text.
#[derive(Debug, Serialize)]
pub struct FuriCell {
    pub text: String,
    pub reading: Option<String>,
}

/// One tooltip definition group: a row of part-of-speech chips followed by
/// the numbered glosses sharing those tags (consecutive senses with the same
/// tag set collapse into one group, the way JMdict lists them). `start` keeps
/// the visible numbering continuous across groups.
#[derive(Debug, Serialize)]
pub struct SenseGroupView {
    pub pos: Vec<String>,
    pub start: usize,
    pub senses: Vec<SenseLineView>,
}

/// One numbered gloss line: `indicates sentence topic — pronounced わ…`.
#[derive(Debug, Serialize)]
pub struct SenseLineView {
    pub gloss: String,
    pub info: Option<String>,
}

/// One conjugation line: `Continuative (~te) of 勉強する【べんきょうする】`.
#[derive(Debug, Serialize)]
pub struct ConjView {
    pub description: String,
    pub base: Option<String>,
}

/// One histogram bar, width precomputed so the template only sets a style.
#[derive(Debug, Serialize)]
pub struct BarRow {
    pub label: String,
    pub count: usize,
    /// 0–100, relative to the tallest bar in the same histogram.
    pub percent: usize,
    /// Extra `bar-fill` class (`lvl5`…`lvl1`, `none`, or empty).
    pub class: String,
}

/// Build the reader page's view-model from a v2 document. `parse_time` is
/// how long the segmentation (the `v2_document` call) took.
pub fn reader_view(
    document: &V2Document,
    levels: &Levels,
    parse_time: std::time::Duration,
) -> ReaderView {
    let no_entries = BTreeMap::new();
    let entries = document.entries.as_ref().unwrap_or(&no_entries);

    let chunks = group_chunks(&document.tokens, entries, levels);

    let words: Vec<&TokenView> = chunks
        .iter()
        .flat_map(|chunk| &chunk.tokens)
        .filter(|token| !token.gap)
        .collect();
    let word_count = words.len();

    let kanji_chars: Vec<char> = words
        .iter()
        .flat_map(|token| token.text.chars())
        .filter(|character| is_kanji(*character))
        .collect();

    ReaderView {
        text: document.text.clone(),
        word_count,
        kanji_count: kanji_chars.len(),
        jlpt_rows: jlpt_rows(&words),
        grade_rows: grade_rows(&kanji_chars, levels),
        wk_rows: wk_rows(&kanji_chars, levels),
        parse_ms: format!("{:.1}", parse_time.as_secs_f64() * 1000.0),
        chunks,
    }
}

/// Group consecutive tokens sharing a `compound` id into one chunk;
/// everything else becomes a singleton chunk.
fn group_chunks(
    tokens: &[V2Token],
    entries: &BTreeMap<i32, V2Entry>,
    levels: &Levels,
) -> Vec<Chunk> {
    let mut chunks: Vec<Chunk> = Vec::new();
    let mut open_compound: Option<u32> = None;

    for token in tokens {
        let view = token_view(token, entries, levels);
        match token.compound {
            Some(id) if open_compound == Some(id) => {
                chunks
                    .last_mut()
                    .expect("open compound implies a previous chunk")
                    .tokens
                    .push(view);
            }
            Some(id) => {
                open_compound = Some(id);
                chunks.push(Chunk { compound: true, tokens: vec![view] });
            }
            None => {
                open_compound = None;
                chunks.push(Chunk { compound: false, tokens: vec![view] });
            }
        }
    }
    chunks
}

/// Shape one v2 token for the templates.
fn token_view(token: &V2Token, entries: &BTreeMap<i32, V2Entry>, levels: &Levels) -> TokenView {
    if token.gap {
        return TokenView {
            gap: true,
            text: token.text.clone(),
            reading: None,
            romanization: token.romanization.clone(),
            furigana: Vec::new(),
            gloss: None,
            sense_groups: Vec::new(),
            extra_senses: 0,
            conjugation: Vec::new(),
            suffix: None,
            is_particle: false,
            seq: None,
            jlpt: None,
        };
    }

    let all_senses: &[V2Sense] = token
        .entry
        .and_then(|seq| entries.get(&seq))
        .map(|entry| entry.senses.as_slice())
        .unwrap_or_default();

    let is_particle = all_senses
        .iter()
        .any(|sense| sense.pos.iter().any(|pos_tag| pos_tag == "prt"));

    let shown = &all_senses[..all_senses.len().min(MAX_TOOLTIP_SENSES)];
    let sense_groups = group_senses(shown);
    let extra_senses = all_senses.len() - shown.len();
    let gloss = shown.first().map(|sense| sense.gloss.join("; "));

    let conjugation: Vec<ConjView> = token
        .conjugation
        .iter()
        .map(|analysis| ConjView {
            description: analysis.description.clone(),
            base: base_display(analysis.base_form.as_deref(), analysis.base_reading.as_deref()),
        })
        .collect();

    let reading = token
        .reading
        .clone()
        .filter(|reading| reading != &token.text);

    TokenView {
        gap: false,
        furigana: furigana_cells(token, reading.as_deref()),
        gloss,
        sense_groups,
        extra_senses,
        conjugation,
        suffix: token
            .suffix
            .as_ref()
            .map(|suffix| suffix.description.clone()),
        is_particle,
        seq: token.entry,
        jlpt: levels.jlpt_word(&token.text, reading.as_deref().unwrap_or(&token.text)),
        text: token.text.clone(),
        romanization: token.romanization.clone(),
        reading,
    }
}

/// Fold consecutive senses with identical part-of-speech tags into one
/// tooltip group; `start` is the 1-based number of the group's first sense.
fn group_senses(senses: &[V2Sense]) -> Vec<SenseGroupView> {
    let mut groups: Vec<SenseGroupView> = Vec::new();
    for (index, sense) in senses.iter().enumerate() {
        let line = SenseLineView {
            gloss: sense.gloss.join("; "),
            info: sense.info.clone(),
        };
        match groups.last_mut() {
            Some(group) if group.pos == sense.pos => group.senses.push(line),
            _ => groups.push(SenseGroupView {
                pos: sense.pos.clone(),
                start: index + 1,
                senses: vec![line],
            }),
        }
    }
    groups
}

/// Ruby cells for a word. The aligned segments come from the v2 document;
/// when alignment was skipped or failed but the word does contain kanji,
/// fall back to one ruby spanning the whole word so it still renders.
fn furigana_cells(token: &V2Token, reading: Option<&str>) -> Vec<FuriCell> {
    if !token.furigana.is_empty() {
        return token
            .furigana
            .iter()
            .map(|cell| FuriCell {
                text: cell.text.clone(),
                reading: cell.reading.clone(),
            })
            .collect();
    }
    let whole_word_reading = reading
        .filter(|_| token.text.chars().any(is_kanji))
        .map(str::to_owned);
    vec![FuriCell {
        text: token.text.clone(),
        reading: whole_word_reading,
    }]
}

/// `食べる` + `たべる` → `食べる【たべる】`; kana-only bases show once.
fn base_display(base_form: Option<&str>, base_reading: Option<&str>) -> Option<String> {
    match (base_form, base_reading) {
        (Some(form), Some(reading)) if form != reading => {
            Some(format!("{form}【{reading}】"))
        }
        (Some(form), _) => Some(form.to_owned()),
        (None, Some(reading)) => Some(reading.to_owned()),
        (None, None) => None,
    }
}

/// Word-JLPT histogram: always all six rows, easiest first, `—` = unlisted.
fn jlpt_rows(words: &[&TokenView]) -> Vec<BarRow> {
    let buckets: &[(Option<u8>, &str, &str)] = &[
        (Some(5), "N5", "lvl5"),
        (Some(4), "N4", "lvl4"),
        (Some(3), "N3", "lvl3"),
        (Some(2), "N2", "lvl2"),
        (Some(1), "N1", "lvl1"),
        (None, "—", "none"),
    ];
    let counts: Vec<usize> = buckets
        .iter()
        .map(|(level, _, _)| words.iter().filter(|token| token.jlpt == *level).count())
        .collect();
    bar_rows(buckets.iter().zip(counts).map(|((_, label, class), count)| {
        (label.to_string(), class.to_string(), count)
    }))
}

/// Kanji histogram by Joyo grade (1–6 elementary, 8 secondary, then
/// non-Joyo); rows with zero count are dropped.
fn grade_rows(kanji_chars: &[char], levels: &Levels) -> Vec<BarRow> {
    let grades: Vec<Option<i32>> = kanji_chars
        .iter()
        .map(|character| levels.kanji(*character).and_then(|stats| stats.grade))
        .collect();
    let buckets: Vec<(String, String, usize)> = [1, 2, 3, 4, 5, 6, 8]
        .iter()
        .map(|grade| {
            (
                format!("Grade {grade}"),
                String::new(),
                grades.iter().filter(|found| **found == Some(*grade)).count(),
            )
        })
        .chain(std::iter::once((
            "non-Jōyō".to_owned(),
            "none".to_owned(),
            grades.iter().filter(|found| found.is_none()).count(),
        )))
        .filter(|(_, _, count)| *count > 0)
        .collect();
    bar_rows(buckets.into_iter())
}

/// Kanji histogram by WaniKani level, in ten-level buckets; rows with zero
/// count are dropped.
fn wk_rows(kanji_chars: &[char], levels: &Levels) -> Vec<BarRow> {
    let wk_levels: Vec<Option<i32>> = kanji_chars
        .iter()
        .map(|character| levels.kanji(*character).and_then(|stats| stats.wk_level))
        .collect();
    let buckets: Vec<(String, String, usize)> = (0..6)
        .map(|bucket| {
            let low = bucket * 10 + 1;
            let high = low + 9;
            (
                format!("Lv {low}–{high}"),
                String::new(),
                wk_levels
                    .iter()
                    .filter(|found| found.is_some_and(|level| level >= low && level <= high))
                    .count(),
            )
        })
        .chain(std::iter::once((
            "not on WK".to_owned(),
            "none".to_owned(),
            wk_levels.iter().filter(|found| found.is_none()).count(),
        )))
        .filter(|(_, _, count)| *count > 0)
        .collect();
    bar_rows(buckets.into_iter())
}

/// Attach percent widths relative to the tallest bar.
fn bar_rows(buckets: impl Iterator<Item = (String, String, usize)>) -> Vec<BarRow> {
    let collected: Vec<(String, String, usize)> = buckets.collect();
    let max = collected
        .iter()
        .map(|(_, _, count)| *count)
        .max()
        .unwrap_or(0)
        .max(1);
    collected
        .into_iter()
        .map(|(label, class, count)| BarRow {
            label,
            count,
            percent: count * 100 / max,
            class,
        })
        .collect()
}

/// Kanji + the iteration/abbreviation marks kaniran treats as kanji — same
/// range as the serializer's private `is_kanji_char` (keep in sync).
fn is_kanji(character: char) -> bool {
    matches!(character, '々' | 'ヶ' | '〆' | '\u{4e00}'..='\u{9faf}')
}
