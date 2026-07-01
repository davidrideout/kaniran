//! v2 JSON — a flat, named-field view of a segmentation result
//!
//! Two things make it cheaper than it looks:
//! - Glosses are pulled on demand. Nothing here stores senses; they are
//!   fetched by [`get_senses_json`] only when [`Detail::Full`] asks, so
//!   [`Detail::Minimal`] does no gloss work at all.
//!
//! The conjugation walk mirrors the data access of
//! [`crate::dict::conj::conj_info_json_star_`] but accumulates a flat
//! `steps` list instead of nesting `via` chains. Field-level parity with the
//! spec is not yet verified against the corpus (the `cli_full` byte-compare any
//! new serializer needs); the structural shape and data sources are.

use std::error::Error;

use serde::Serialize;
use serde_json::Value;

use crate::conn::kani_context::KaniranContext;
use crate::conn::KaniDbError;
use crate::core::kani_romanize_method::KaniRomanizeMethod;
use crate::core::romanize::{join_parts, romanize_word_info, RomanizeStarSegment};
use crate::dict::conj::{select_conjs_and_props, FilterPropsText};
use crate::dict::dao::{ConjProp, WordConjugations};
use crate::dict::grammar::suffix::init::get_suffix_description;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::load::conj_rules::get_conj_description;
use crate::dict::senses::{get_first_pos, get_senses_json};
use crate::dict::word_info::{WordInfo, WordInfoKana, WordInfoSeq, WordInfoType};
use crate::dict::word_info_str::reading_str_seq;

/// Render `input` as flat v2 JSON at the given `detail` level.
pub(super) fn render(
    ctx: &KaniranContext,
    input: &str,
    method: KaniRomanizeMethod<'_>,
    limit: usize,
    detail: Detail,
    include_paths: bool,
) -> Result<String, Box<dyn Error>> {
    let result = super::segment(ctx, input, method, limit)?;
    let document = to_v2(ctx, input, &result, method, detail, include_paths)?;
    Ok(serde_json::to_string(&document)?)
}

/// How much of each word to render.
///
/// The variants are nested supersets: everything `Minimal` emits, `Full`
/// emits too. The difference is the gloss-bearing sections (`senses`,
/// `conjugations`, `alternatives`), which `Minimal` omits — and, with them,
/// the sense lookups that dominate serialization cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    /// Segmentation skeleton plus the cheap conjugation summary
    /// (`conjugation`, `base_form`, `base_reading`). No glosses.
    Minimal,
    /// Every field, including senses and the full `conjugations` array.
    Full,
}

/// Top-level v2 result: the whole input as one flat ordered `words` array.
///
/// `words`/`romanization`/`score` describe the single best segmentation. When
/// the caller opts in (`include_paths`) and the search kept more than one beam
/// path (`limit > 1` and the input is genuinely ambiguous), `paths` carries
/// every reading — `paths[0]` is the same data as the top-level best path, and
/// later entries are the alternatives, each a full flat word list with its own
/// score and romanization. `paths` is omitted entirely otherwise.
#[derive(Debug, Clone, Serialize)]
struct V2Document {
    text: String,
    romanization: String,
    score: i32,
    words: Vec<V2Word>,
    #[serde(skip_serializing_if = "Option::is_none")]
    paths: Option<Vec<V2Path>>,
}

/// One full segmentation reading: a complete flat word list with its own score
/// and romanization. Same `words` shape as the top-level document; `paths[0]`
/// equals the top-level best path.
#[derive(Debug, Clone, Serialize)]
struct V2Path {
    score: i32,
    romanization: String,
    words: Vec<V2Word>,
}

/// One word (or gap) in the best segmentation path.
///
/// Field order matches the spec's word-object table. Nullable scalars
/// serialize as `null` when absent (always present in both detail levels);
/// the three gloss-bearing arrays are `Option`-wrapped and skipped entirely
/// under [`Detail::Minimal`].
///
/// `Default` (all fields empty/`None`/`0`) backs [`V2Builder::gap_word`], which
/// names only the fields a gap sets and leaves the rest at their defaults.
#[derive(Debug, Clone, Default, Serialize)]
struct V2Word {
    text: String,
    reading: String,
    romanization: String,
    start: usize,
    end: usize,
    seq: Option<i32>,
    score: i32,
    pos: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    senses: Option<Vec<V2Sense>>,
    conjugation: Option<String>,
    base_form: Option<String>,
    base_reading: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    conjugations: Option<Vec<V2Conjugation>>,
    suffix: Option<String>,
    counter: Option<V2Counter>,
    compound_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alternatives: Option<Vec<V2Word>>,
    alt_readings: Vec<V2AltReading>,
}

/// A dictionary sense, with v1's `pos`/`field` bracket decoration stripped.
#[derive(Debug, Clone, Serialize)]
struct V2Sense {
    pos: String,
    gloss: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
}

/// One conjugation analysis: a root entry plus the ordered steps that produce
/// the surface form. The flat replacement for v1's recursive `conj`/`via`.
#[derive(Debug, Clone, Serialize)]
struct V2Conjugation {
    seq: i32,
    steps: Vec<V2Step>,
    base_form: Option<String>,
    base_reading: Option<String>,
    reading_matched: bool,
    senses: Vec<V2Sense>,
}

/// One conjugation step. `steps[0]` applies to the dictionary form; the last
/// step produces the surface form.
#[derive(Debug, Clone, Serialize)]
struct V2Step {
    form: String,
    pos: String,
    negative: bool,
    formal: bool,
}

/// Number+counter info, with v1's `"Value: "` prefix stripped.
#[derive(Debug, Clone, Serialize)]
struct V2Counter {
    value: String,
    ordinal: bool,
}

/// An additional kana reading of the same entry.
#[derive(Debug, Clone, Serialize)]
struct V2AltReading {
    reading: String,
    romanization: String,
}

/// `get_senses_json` takes an optional reading-getter closure; v2 always uses
/// the default reading, so it needs a concrete type to pin the `None`.
type NoGetter = fn() -> Result<Option<KaniWordDispatchEnum>, KaniDbError>;
const NO_GETTER: Option<NoGetter> = None;

/// Build the v2 document from a `romanize*` result.
///
/// `text` is the original input (the top-level `text` field). The top-level
/// `words`/`score`/`romanization` come from the best path (beam rank 0). When
/// more than one beam path survives — the search kept several and at least one
/// word segment has alternatives — every reading is rendered into `paths`.
/// `method` romanizes compound members and alternate readings, whose
/// romanizations `romanize*` does not precompute.
fn to_v2<P>(
    ctx: &KaniranContext,
    text: &str,
    segments: &[RomanizeStarSegment<P>],
    method: KaniRomanizeMethod<'_>,
    detail: Detail,
    include_paths: bool,
) -> Result<V2Document, KaniDbError> {
    let builder = V2Builder {
        ctx,
        method,
        detail,
    };

    // Beam ranks available = the most alternatives any one word segment kept.
    // The search already caps this at `limit`; Misc gaps add no alternatives.
    let n_paths = segments
        .iter()
        .map(|segment| match segment {
            RomanizeStarSegment::Word(alternatives) => alternatives.len(),
            RomanizeStarSegment::Misc(_) => 0,
        })
        .max()
        .unwrap_or(0);

    // Build the best reading once; it always supplies the top-level fields.
    let best = build_path(&builder, segments, 0)?;

    // Emit `paths` only when the caller opts in (`include_paths`) and the
    // search kept more than one reading. The best path appears both at top
    // level and as paths[0], so it is cloned once.
    let paths = if include_paths && n_paths > 1 {
        let mut paths = Vec::with_capacity(n_paths);
        paths.push(best.clone());
        for rank in 1..n_paths {
            paths.push(build_path(&builder, segments, rank)?);
        }
        Some(paths)
    } else {
        None
    };

    Ok(V2Document {
        text: text.to_owned(),
        romanization: best.romanization,
        score: best.score,
        words: best.words,
        paths,
    })
}

/// Render one beam path (rank `rank`) across all segments into a flat word
/// list. Each word segment contributes its `rank`-th alternative, clamped to
/// its last when it kept fewer (shorter beams reuse their best). Misc gaps pass
/// through on every path. `compound_id` restarts per path, so ids are
/// path-local.
fn build_path<P>(
    builder: &V2Builder<'_>,
    segments: &[RomanizeStarSegment<P>],
    rank: usize,
) -> Result<V2Path, KaniDbError> {
    let mut words = Vec::new();
    let mut romaji_parts: Vec<String> = Vec::new();
    let mut base = 0usize; // running char offset into the normalized stream
    let mut score_total = 0i32;
    let mut compound_id = 0u32;

    for segment in segments {
        match segment {
            RomanizeStarSegment::Misc(misc) => {
                words.push(builder.gap_word(misc, base));
                romaji_parts.push(misc.clone());
                base += misc.chars().count();
            }
            RomanizeStarSegment::Word(alternatives) => {
                if alternatives.is_empty() {
                    continue;
                }
                let index = rank.min(alternatives.len() - 1);
                let (word_list, score) = &alternatives[index];
                score_total += *score;
                let mut chunk_len = 0usize;
                for (romanized, word_info, _) in word_list {
                    romaji_parts.push(romanized.clone());
                    chunk_len = chunk_len.max(word_info.end.unwrap_or(0));
                    builder.explode_word(
                        word_info,
                        romanized,
                        base,
                        &mut compound_id,
                        &mut words,
                    )?;
                }
                base += chunk_len;
            }
        }
    }

    Ok(V2Path {
        score: score_total,
        romanization: join_parts(&romaji_parts),
        words,
    })
}

/// The constants threaded through every word: the backend handle, the
/// romanization method, and the detail level.
struct V2Builder<'a> {
    ctx: &'a KaniranContext,
    method: KaniRomanizeMethod<'a>,
    detail: Detail,
}

/// What a word inherits from its place in the segmentation: its character
/// span, compound group, suffix flag, and score. Tied analyses of one span
/// share a placement, so it threads through `build_word` unchanged.
#[derive(Debug, Clone, Copy)]
struct WordPlacement {
    start: usize,
    end: usize,
    compound_id: Option<u32>,
    is_suffix: bool,
    score: i32,
}

impl V2Builder<'_> {
    /// Emit a top-level word as one or more `V2Word`s: a compound explodes
    /// into adjacent members sharing a fresh `compound_id`; everything else is
    /// one word.
    fn explode_word(
        &self,
        word_info: &WordInfo,
        romanized: &str,
        base: usize,
        compound_id: &mut u32,
        out: &mut Vec<V2Word>,
    ) -> Result<(), KaniDbError> {
        let score = word_info.score.unwrap_or(0);
        // An alternative wrapper carries components but is one word at one span,
        // not a compound; route it here so `build_word` promotes the best entry
        // and fills `alternatives`, instead of exploding the ties as members
        // with fabricated advancing spans.
        if word_info.components.is_empty() || word_info.alternative {
            let placement = WordPlacement {
                start: base + word_info.start.unwrap_or(0),
                end: base + word_info.end.unwrap_or(0),
                compound_id: None,
                is_suffix: false,
                score,
            };
            out.push(self.build_word(word_info, romanized.to_owned(), placement)?);
            return Ok(());
        }

        // Members carry the compound's score (v1 gives them none of their
        // own), and their romanization is computed from each member's reading.
        // They carry no offsets of their own either, so distribute the
        // compound's span across them by each member's character length.
        *compound_id += 1;
        let id = *compound_id;
        let mut member_start = base + word_info.start.unwrap_or(0);
        for member in &word_info.components {
            let member_end = member_start + member.text.chars().count();
            let placement = WordPlacement {
                start: member_start,
                end: member_end,
                compound_id: Some(id),
                is_suffix: !member.primary,
                score,
            };
            let member_romaji = romanize_word_info(member, self.method);
            out.push(self.build_word(member, member_romaji, placement)?);
            member_start = member_end;
        }
        Ok(())
    }

    /// Build one `V2Word` from a single `WordInfo`.
    fn build_word(
        &self,
        word_info: &WordInfo,
        romanization: String,
        placement: WordPlacement,
    ) -> Result<V2Word, KaniDbError> {
        // A tied-analyses wrapper: promote the best entry into the word itself
        // and place the rest in `alternatives` (bounded at one level — their
        // own `alternatives` stays empty). All tied entries share the span.
        if word_info.alternative && !word_info.components.is_empty() {
            let (best, rest) = word_info.components.split_first().expect("non-empty");
            let mut word =
                self.build_word(best, romanize_word_info(best, self.method), placement)?;
            if self.detail == Detail::Full {
                let mut alts = Vec::with_capacity(rest.len());
                for alt in rest {
                    let mut alt_word =
                        self.build_word(alt, romanize_word_info(alt, self.method), placement)?;
                    alt_word.alternatives = None; // bound nesting at one level
                    alts.push(alt_word);
                }
                word.alternatives = Some(alts);
            }
            return Ok(word);
        }

        let (reading, alt_readings) = self.split_kana_readings(word_info);

        let conj = self.analyze_conjugations(word_info)?;
        // Synthetic seq for a conjugated form; real root seq once resolved.
        let raw_seq = single_seq(&word_info.seq);
        let seq = conj.as_ref().map(|summary| summary.seq).or(raw_seq);

        // Full only: the word's own senses, or a copy of the root entry's
        // senses when the word is a conjugated form.
        let senses = if self.detail == Detail::Full {
            let list = match &conj {
                Some(summary) => summary.senses.clone(),
                None => match seq {
                    Some(seq) => {
                        reshape_senses(&get_senses_json(self.ctx, seq, &[], None, NO_GETTER)?)
                    }
                    None => Vec::new(),
                },
            };
            Some(list)
        } else {
            None
        };

        // pos: the surface step's pos for a conjugated form; otherwise the
        // first pos tag of the entry's first sense. Full already has the senses
        // in hand, so it reuses them; Minimal has none (it skips gloss fetches)
        // and fetches the pos alone, so every word still carries a pos.
        let pos = match &conj {
            Some(summary) => summary.pos.clone(),
            None => match (&senses, seq) {
                (Some(list), _) => first_sense_pos(list),
                (None, Some(seq)) => get_first_pos(self.ctx, seq)?,
                (None, None) => None,
            },
        };

        let conjugations = (self.detail == Detail::Full).then(|| {
            conj.as_ref()
                .map(|summary| summary.analyses.clone())
                .unwrap_or_default()
        });

        let suffix = if placement.is_suffix {
            seq.and_then(|seq| get_suffix_description(self.ctx, seq).map(str::to_owned))
        } else {
            None
        };

        let counter = word_info
            .counter
            .as_ref()
            .map(|(value, ordinal)| V2Counter {
                value: strip_value_prefix(value),
                ordinal: *ordinal,
            });

        Ok(V2Word {
            text: word_info.text.clone(),
            reading,
            romanization,
            start: placement.start,
            end: placement.end,
            seq,
            score: placement.score,
            pos,
            senses,
            conjugation: conj.as_ref().map(|summary| summary.display.clone()),
            base_form: conj.as_ref().and_then(|summary| summary.base_form.clone()),
            base_reading: conj
                .as_ref()
                .and_then(|summary| summary.base_reading.clone()),
            conjugations,
            suffix,
            counter,
            compound_id: placement.compound_id,
            alternatives: (self.detail == Detail::Full).then(Vec::new),
            alt_readings,
        })
    }

    /// A word with no dictionary match: punctuation, latin, unrecognized spans.
    fn gap_word(&self, text: &str, base: usize) -> V2Word {
        let len = text.chars().count();
        let full = self.detail == Detail::Full;
        V2Word {
            text: text.to_owned(),
            reading: text.to_owned(),
            romanization: text.to_owned(),
            start: base,
            end: base + len,
            senses: full.then(Vec::new),
            conjugations: full.then(Vec::new),
            alternatives: full.then(Vec::new),
            ..Default::default()
        }
    }

    /// First kana reading as the word's `reading`; any further readings become
    /// `alt_readings` (with their own romanization).
    fn split_kana_readings(&self, word_info: &WordInfo) -> (String, Vec<V2AltReading>) {
        match &word_info.kana {
            None => (strip_marks(&word_info.text), Vec::new()),
            Some(WordInfoKana::Single(kana)) => (strip_marks(kana), Vec::new()),
            Some(WordInfoKana::Multi(list)) => {
                let mut readings = list.iter().flatten().filter_map(first_kana);
                let reading = readings
                    .next()
                    .map(|kana| strip_marks(&kana))
                    .unwrap_or_else(|| strip_marks(&word_info.text));
                let alts = readings
                    .map(|kana| {
                        let clean = strip_marks(&kana);
                        let romanization = romanize_kana(&clean, self.method);
                        V2AltReading {
                            reading: clean,
                            romanization,
                        }
                    })
                    .collect();
                (reading, alts)
            }
        }
    }

    /// Resolve a word's conjugation chain, or `None` if it is a dictionary form.
    fn analyze_conjugations(
        &self,
        word_info: &WordInfo,
    ) -> Result<Option<ConjSummary>, KaniDbError> {
        let Some(seq) = single_seq(&word_info.seq) else {
            return Ok(None);
        };
        let text = match &word_info.true_text {
            Some(true_text) => FilterPropsText::One(true_text),
            None => FilterPropsText::None,
        };
        let flats = self.flatten_conj(seq, word_info.conjugations.as_ref(), text)?;
        let Some(first) = flats.first() else {
            return Ok(None);
        };

        let pos = first.steps.last().map(|step| step.pos.clone());
        let display = build_display(&first.steps);
        Ok(Some(ConjSummary {
            seq: first.seq,
            pos,
            display,
            base_form: first.base_form.clone(),
            base_reading: first.base_reading.clone(),
            senses: first.senses.clone().unwrap_or_default(),
            analyses: flats.iter().map(to_v2_conjugation).collect(),
        }))
    }

    /// Walk the conjugation rows for `seq`, flattening each via chain into one
    /// `FlatConj` whose `steps` run root-to-surface.
    ///
    /// Mirrors the data access of `conj_info_json_star_`: the base level
    /// (`seq_via == None`) holds the root `seq_from` and its senses; a `via`
    /// level recurses and appends its own step after the deeper ones.
    fn flatten_conj(
        &self,
        seq: i32,
        conjugations: Option<&WordConjugations>,
        text: FilterPropsText<'_>,
    ) -> Result<Vec<FlatConj>, KaniDbError> {
        let mut out = Vec::new();
        let mut via_used: Vec<i32> = Vec::new();

        for (conj, props, _key) in select_conjs_and_props(self.ctx, seq, conjugations, text)? {
            if let Some(via) = conj.seq_via {
                if via_used.contains(&via) {
                    continue;
                }
            }
            let surface_steps: Vec<V2Step> = props.iter().map(step_from_prop).collect();

            match conj.seq_via {
                None => {
                    let (base_form, base_reading) =
                        split_reading(reading_str_seq(self.ctx, conj.seq_from)?);
                    let senses = if self.detail == Detail::Full {
                        let pos_list: Vec<String> =
                            props.iter().map(|prop| prop.pos.clone()).collect();
                        let json =
                            get_senses_json(self.ctx, conj.seq_from, &pos_list, None, NO_GETTER)?;
                        Some(reshape_senses(&json))
                    } else {
                        None
                    };
                    out.push(FlatConj {
                        seq: conj.seq_from,
                        steps: surface_steps,
                        base_form,
                        base_reading,
                        reading_matched: true,
                        senses,
                    });
                }
                Some(via) => {
                    let deeper = self.flatten_conj(via, None, FilterPropsText::None)?;
                    for chain in deeper {
                        let mut steps = chain.steps;
                        steps.extend(surface_steps.iter().cloned());
                        out.push(FlatConj { steps, ..chain });
                    }
                    via_used.push(via);
                }
            }
        }

        Ok(out)
    }
}

/// The derived per-word conjugation summary plus, in `Full`, every analysis.
struct ConjSummary {
    seq: i32,
    pos: Option<String>,
    display: String,
    base_form: Option<String>,
    base_reading: Option<String>,
    senses: Vec<V2Sense>,
    analyses: Vec<V2Conjugation>,
}

/// One flattened analysis as it accumulates through the via chain.
struct FlatConj {
    seq: i32,
    steps: Vec<V2Step>,
    base_form: Option<String>,
    base_reading: Option<String>,
    reading_matched: bool,
    senses: Option<Vec<V2Sense>>,
}

fn to_v2_conjugation(flat: &FlatConj) -> V2Conjugation {
    V2Conjugation {
        seq: flat.seq,
        steps: flat.steps.clone(),
        base_form: flat.base_form.clone(),
        base_reading: flat.base_reading.clone(),
        reading_matched: flat.reading_matched,
        senses: flat.senses.clone().unwrap_or_default(),
    }
}

fn step_from_prop(prop: &ConjProp) -> V2Step {
    V2Step {
        form: get_conj_description(prop.conj_type)
            .unwrap_or_default()
            .to_owned(),
        pos: prop.pos.clone(),
        negative: prop.neg == Some(true),
        formal: prop.fml == Some(true),
    }
}

/// The display string: surface form first, deeper steps appended with
/// `" via "`, negation/formality of the surface step trailing.
fn build_display(steps: &[V2Step]) -> String {
    let Some((surface, deeper)) = steps.split_last() else {
        return String::new();
    };
    let mut forms = Vec::with_capacity(steps.len());
    forms.push(surface.form.clone());
    forms.extend(deeper.iter().rev().map(|step| step.form.clone()));
    let mut display = forms.join(" via ");
    if surface.negative {
        display.push_str(", negative");
    }
    if surface.formal {
        display.push_str(", formal");
    }
    display
}

/// Split a composite reading (`"食べる 【たべる】"`) into `(base_form,
/// base_reading)`; a kana-only reading is its own base form.
fn split_reading(reading: Option<String>) -> (Option<String>, Option<String>) {
    let Some(reading) = reading else {
        return (None, None);
    };
    match reading.find('【') {
        Some(open) => {
            let kanji = reading[..open].trim().to_owned();
            let kana = reading[open + '【'.len_utf8()..]
                .trim_end_matches('】')
                .trim()
                .to_owned();
            (Some(kanji), Some(kana))
        }
        None => (Some(reading.clone()), Some(reading)),
    }
}

fn first_kana(kana: &WordInfoKana) -> Option<String> {
    match kana {
        WordInfoKana::Single(text) => Some(text.clone()),
        WordInfoKana::Multi(list) => list.iter().flatten().find_map(first_kana),
    }
}

/// Romanize a bare kana string via a throwaway kana word-info.
fn romanize_kana(kana: &str, method: KaniRomanizeMethod<'_>) -> String {
    let word_info = WordInfo {
        kind: WordInfoType::Kana,
        text: kana.to_owned(),
        ..WordInfo::default()
    };
    romanize_word_info(&word_info, method)
}

/// Reshape v1 sense JSON into `V2Sense`, stripping the `[pos]` / `{field}`
/// decoration. The sense data itself is unchanged.
fn reshape_senses(senses: &[Value]) -> Vec<V2Sense> {
    senses
        .iter()
        .map(|sense| V2Sense {
            pos: sense
                .get("pos")
                .and_then(Value::as_str)
                .map(strip_brackets)
                .unwrap_or_default(),
            gloss: sense
                .get("gloss")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            info: sense.get("info").and_then(Value::as_str).map(str::to_owned),
            field: sense.get("field").and_then(Value::as_str).map(strip_braces),
        })
        .collect()
}

fn first_sense_pos(senses: &[V2Sense]) -> Option<String> {
    senses
        .first()
        .map(|sense| sense.pos.split(',').next().unwrap_or_default().to_owned())
}

fn single_seq(seq: &Option<WordInfoSeq>) -> Option<i32> {
    match seq {
        Some(WordInfoSeq::Single(seq)) => Some(*seq),
        _ => None,
    }
}

/// Drop the zero-width markers v1 embeds in kana to position romanization
/// boundaries, so v2 readings are clean display strings.
fn strip_marks(reading: &str) -> String {
    reading
        .chars()
        .filter(|char| !matches!(char, '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{feff}'))
        .collect()
}

fn strip_brackets(pos: &str) -> String {
    pos.trim_start_matches('[').trim_end_matches(']').to_owned()
}

fn strip_braces(field: &str) -> String {
    field
        .trim_start_matches('{')
        .trim_end_matches('}')
        .to_owned()
}

fn strip_value_prefix(value: &str) -> String {
    value.strip_prefix("Value: ").unwrap_or(value).to_owned()
}

#[cfg(test)]
mod tests {
    //! Logic-only tests for the v2 reshaping helpers, plus one DB-backed
    //! regression (`v2_minimal_populates_pos_for_unconjugated_words`) that
    //! renders against the configured snapshot. Every expected value is real
    //! pipeline output.
    use super::*;
    use crate::core::methods::{hepburn_traditional, RomanizationMethod};
    use serde_json::json;

    /// A conjugation step with a form name; `pos` is irrelevant to display.
    fn step(form: &str, negative: bool, formal: bool) -> V2Step {
        V2Step {
            form: form.to_owned(),
            pos: "v1".to_owned(),
            negative,
            formal,
        }
    }

    #[test]
    fn build_display_single_step_is_just_the_form() {
        // 食べ → "Continuative (~i)" (spec, 食べたくなかった example).
        let steps = [step("Continuative (~i)", false, false)];
        assert_eq!(build_display(&steps), "Continuative (~i)");
    }

    #[test]
    fn build_display_via_chain_appends_deeper_steps_reversed() {
        // 食べさせられ: steps run root→surface [Causative-Passive, Continuative],
        // and display is surface-first, deeper trailing after " via " (spec).
        let steps = [
            step("Causative-Passive", false, false),
            step("Continuative (~i)", false, false),
        ];
        assert_eq!(
            build_display(&steps),
            "Continuative (~i) via Causative-Passive"
        );
    }

    #[test]
    fn build_display_marks_negative_and_formal_on_the_surface_step() {
        // たくなかった → "Past (~ta), negative"; いました → "Past (~ta), formal".
        assert_eq!(
            build_display(&[step("Past (~ta)", true, false)]),
            "Past (~ta), negative"
        );
        assert_eq!(
            build_display(&[step("Past (~ta)", false, true)]),
            "Past (~ta), formal"
        );
    }

    #[test]
    fn build_display_empty_steps_is_empty_string() {
        assert_eq!(build_display(&[]), "");
    }

    #[test]
    fn split_reading_splits_composite_into_kanji_and_kana() {
        // The conjugation base reading arrives as "食べる 【たべる】".
        let (base_form, base_reading) = split_reading(Some("食べる 【たべる】".to_owned()));
        assert_eq!(base_form.as_deref(), Some("食べる"));
        assert_eq!(base_reading.as_deref(), Some("たべる"));
    }

    #[test]
    fn split_reading_kana_only_is_its_own_base_form() {
        // たい has no kanji head, so it is both the form and the reading.
        let (base_form, base_reading) = split_reading(Some("たい".to_owned()));
        assert_eq!(base_form.as_deref(), Some("たい"));
        assert_eq!(base_reading.as_deref(), Some("たい"));
    }

    #[test]
    fn split_reading_none_yields_none_pair() {
        assert_eq!(split_reading(None), (None, None));
    }

    #[test]
    fn strip_marks_drops_zero_width_markers() {
        // The は particle reading carries a leading zero-width non-joiner; the
        // こんにちは reading carries one mid-string. v2 readings are clean.
        assert_eq!(strip_marks("\u{200c}は"), "は");
        assert_eq!(strip_marks("こんにち\u{200c}は"), "こんにちは");
    }

    #[test]
    fn reshape_senses_strips_decoration_and_carries_info_and_field() {
        // Real 世界 senses: plain entry, then one with {Buddh} field + info note.
        let input = [
            json!({ "pos": "[n]", "gloss": "the world; society; the universe" }),
            json!({
                "pos": "[n]",
                "gloss": "realm governed by one Buddha; space",
                "field": "{Buddh}",
                "info": "original meaning"
            }),
        ];
        let senses = reshape_senses(&input);

        assert_eq!(senses.len(), 2);
        assert_eq!(senses[0].pos, "n");
        assert_eq!(senses[0].gloss, "the world; society; the universe");
        assert_eq!(senses[0].info, None);
        assert_eq!(senses[0].field, None);

        assert_eq!(senses[1].pos, "n");
        assert_eq!(senses[1].field.as_deref(), Some("Buddh"));
        assert_eq!(senses[1].info.as_deref(), Some("original meaning"));
    }

    #[test]
    fn reshape_senses_keeps_multi_tag_pos_intact() {
        // Only the surrounding brackets are stripped; the comma list stays.
        let input = [json!({ "pos": "[n,adv]", "gloss": "today; this day" })];
        assert_eq!(reshape_senses(&input)[0].pos, "n,adv");
    }

    #[test]
    fn first_sense_pos_takes_the_first_comma_segment_of_the_first_sense() {
        let senses = [
            V2Sense {
                pos: "n,adv".to_owned(),
                gloss: "today; this day".to_owned(),
                info: None,
                field: None,
            },
            V2Sense {
                pos: "pn".to_owned(),
                gloss: "she; her".to_owned(),
                info: None,
                field: None,
            },
        ];
        assert_eq!(first_sense_pos(&senses).as_deref(), Some("n"));
        assert_eq!(first_sense_pos(&[]), None);
    }

    #[test]
    fn strip_value_prefix_removes_the_counter_value_label() {
        // v1 counters arrive as "Value: 35"; v2 exposes the bare number.
        assert_eq!(strip_value_prefix("Value: 35"), "35");
        assert_eq!(strip_value_prefix("35"), "35");
    }

    #[test]
    fn single_seq_only_resolves_a_single_sequence() {
        assert_eq!(single_seq(&Some(WordInfoSeq::Single(1358280))), Some(1358280));
        assert_eq!(single_seq(&None), None);
    }

    fn method() -> KaniRomanizeMethod<'static> {
        KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()))
    }

    /// The `pos` of every word in a rendered document, in order.
    fn word_pos(document: &str) -> Vec<Option<String>> {
        let value: Value = serde_json::from_str(document).unwrap();
        value["words"]
            .as_array()
            .unwrap()
            .iter()
            .map(|word| word["pos"].as_str().map(str::to_owned))
            .collect()
    }

    #[test]
    fn v2_minimal_populates_pos_for_unconjugated_words() {
        // Regression: v2-minimal left `pos` null on every unconjugated word,
        // because it derives pos from senses and Minimal skips gloss fetches.
        // 食べたい = 食べ (conjugated — pos from the conjugation step) + たい
        // (a suffix, unconjugated — pos comes from its first sense). Minimal
        // now fetches the pos alone, so both words carry one and it matches Full.
        let ctx = crate::test_support::shared_ctx();
        let minimal = render(&ctx, "食べたい", method(), 1, Detail::Minimal, false).unwrap();
        let full = render(&ctx, "食べたい", method(), 1, Detail::Full, false).unwrap();

        let minimal_pos = word_pos(&minimal);
        assert_eq!(
            minimal_pos,
            vec![Some("v1".to_owned()), Some("aux-adj".to_owned())],
        );
        assert_eq!(minimal_pos, word_pos(&full));
    }

    /// The surface text of every word in a rendered document, in order.
    fn word_texts(value: &Value) -> Vec<String> {
        value["words"]
            .as_array()
            .unwrap()
            .iter()
            .map(|word| word["text"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn v2_tied_alternatives_collapse_into_one_word() {
        // Regression: a span with several tied dictionary entries (an
        // `alternative` wrapper) was exploded as a bogus compound — the same
        // surface repeated as separate words with fabricated, overlapping spans
        // and a shared compound_id. It must collapse into one word whose ties
        // live in `alternatives`.
        let ctx = crate::test_support::shared_ctx();

        // Kana homophones: がくせい = 学生 (1206900) and 学制 (1761180). Before
        // the fix, がくせい appeared twice; the second ran to char 12 in a
        // 10-char input.
        let document = render(&ctx, "わたしはがくせいです", method(), 1, Detail::Full, false).unwrap();
        let value: Value = serde_json::from_str(&document).unwrap();
        assert_eq!(word_texts(&value), ["わたし", "は", "がくせい", "です"]);

        let gakusei = &value["words"][2];
        assert_eq!(gakusei["seq"].as_i64(), Some(1206900));
        assert_eq!(gakusei["start"].as_u64(), Some(4));
        assert_eq!(gakusei["end"].as_u64(), Some(8));
        assert!(gakusei["compound_id"].is_null());
        let alts = gakusei["alternatives"].as_array().unwrap();
        assert_eq!(alts.len(), 1);
        assert_eq!(alts[0]["seq"].as_i64(), Some(1761180));

        // No span may run past the input — catches the old advancing offsets.
        let input_len = value["text"].as_str().unwrap().chars().count() as u64;
        for word in value["words"].as_array().unwrap() {
            assert!(word["end"].as_u64().unwrap() <= input_len);
        }

        // v2-minimal collapses the tie too (it just omits `alternatives`).
        let minimal = render(&ctx, "わたしはがくせいです", method(), 1, Detail::Minimal, false).unwrap();
        let minimal: Value = serde_json::from_str(&minimal).unwrap();
        assert_eq!(word_texts(&minimal), ["わたし", "は", "がくせい", "です"]);

        // Not kana-specific: 一日 is a kanji homograph — いちにち (1576260) /
        // ついたち (2225040) — and collapses the same way.
        let document = render(&ctx, "一日", method(), 1, Detail::Full, false).unwrap();
        let value: Value = serde_json::from_str(&document).unwrap();
        let words = value["words"].as_array().unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0]["seq"].as_i64(), Some(1576260));
        assert!(words[0]["compound_id"].is_null());
        let alts = words[0]["alternatives"].as_array().unwrap();
        assert_eq!(alts.len(), 1);
        assert_eq!(alts[0]["seq"].as_i64(), Some(2225040));
    }
}
