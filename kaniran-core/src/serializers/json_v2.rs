//! v2 JSON — the tokens + entries view of a segmentation result
//! (`docs/v2-api-spec-pt2.md`).
//!
//! Two flat collections joined by JMdict sequence number: `tokens` say where
//! things are in the text, `entries` say what the dictionary knows about
//! them. Dictionary facts (forms, senses) exist once, in the entry; tokens,
//! conjugation analyses, and tied alternatives all reference entries by id.
//!
//! Token `text` is the verbatim input slice: segmentation runs on the
//! normalized stream, and [`kani_normalize_spanned`] maps each token's
//! normalized span back to the input chars it came from, so concatenating
//! `tokens[].text` reproduces the input exactly.
//!
//! The conjugation walk mirrors the data access of
//! [`crate::dict::conj::conj_info_json_star_`] but accumulates a flat
//! `steps` list instead of nesting `via` chains, and fetches no senses —
//! an analysis points at its root entry instead.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use serde::Serialize;

use crate::characters::kana::NormalizationContext;
use crate::characters::kani_spanned_normalize::{kani_normalize_spanned, KaniNormalizedSpans};
use crate::conn::kani_backend::KaniBackend;
use crate::conn::kani_context::KaniranContext;
use crate::conn::KaniDbError;
use crate::core::kani_romanize_method::KaniRomanizeMethod;
use crate::core::romanize::{join_parts, romanize_word_info, RomanizeStarSegment};
use crate::dict::conj::{select_conjs_and_props, FilterPropsText};
use crate::dict::dao::{ConjProp, KanaText, WordConjugations};
use crate::dict::grammar::suffix::constants::suffix_class;
use crate::dict::grammar::suffix::init::get_suffix_description;
use crate::dict::load::conj_rules::get_conj_description;
use crate::dict::word_info::{WordInfo, WordInfoKana, WordInfoSeq};
use crate::dict::word_info_str::reading_str_seq;
use crate::kanji::matching::{match_readings, MatchedSegment};

/// Render `input` as v2 tokens + entries JSON, shaped by `options`.
pub(super) fn render(
    ctx: &KaniranContext,
    input: &str,
    method: KaniRomanizeMethod<'_>,
    limit: usize,
    options: V2Options,
) -> Result<String, Box<dyn Error>> {
    let result = super::segment(ctx, input, method, limit)?;
    let document = to_v2(ctx, input, &result, method, options)?;
    Ok(serde_json::to_string(&document)?)
}

/// The v2 document toggles. Each drops one optional section without touching
/// the rest; `Default` matches the HTTP API defaults.
#[derive(Debug, Clone, Copy)]
pub struct V2Options {
    /// Render `paths` (every kept beam reading). Even when set, `paths`
    /// appears only if the search kept more than one reading — `limit > 1`
    /// and a genuinely ambiguous input.
    pub include_paths: bool,
    /// Render the `entries` table.
    pub include_entries: bool,
    /// Run the ruby alignment: `furigana` on tokens and on entry kanji forms.
    pub include_furigana: bool,
}

impl Default for V2Options {
    fn default() -> Self {
        Self {
            include_paths: false,
            include_entries: true,
            include_furigana: true,
        }
    }
}

/// Top-level v2 result: the segmentation as one flat ordered `tokens` array,
/// plus the `entries` those tokens reference, keyed by JMdict sequence
/// number. `paths` appears only when the caller opts in (`include_paths`)
/// and the search kept more than one reading; all paths share the one
/// entries table.
#[derive(Debug, Clone, Serialize)]
struct V2Document {
    text: String,
    romanization: String,
    score: i32,
    tokens: Vec<V2Token>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entries: Option<BTreeMap<i32, V2Entry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    paths: Option<Vec<V2Path>>,
}

/// One full segmentation reading. `paths[0]` equals the top-level best path.
#[derive(Debug, Clone, Serialize)]
struct V2Path {
    score: i32,
    romanization: String,
    tokens: Vec<V2Token>,
}

/// One token — a word or a gap — in a segmentation path. Absent means
/// empty: null/empty/false fields are omitted, so a gap token is three keys
/// and a particle five.
#[derive(Debug, Clone, Default, Serialize)]
struct V2Token {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reading: Option<String>,
    romanization: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    furigana: Vec<V2Furigana>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entry: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<i32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    conjugation: Vec<V2Analysis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    compound: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suffix: Option<V2Suffix>,
    #[serde(skip_serializing_if = "Option::is_none")]
    counter: Option<V2Counter>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    alternatives: Vec<V2Alternative>,
    #[serde(skip_serializing_if = "is_false")]
    gap: bool,
}

/// One ruby segment: `reading` is present only over kanji runs, and the
/// segment `text`s concatenate to the owning token's (or form's) text.
#[derive(Debug, Clone, Serialize)]
struct V2Furigana {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reading: Option<String>,
}

/// One conjugation analysis: the root entry plus the ordered steps that
/// produce the surface form. `base_reading` is the specific root reading
/// this derivation used — not derivable from the entry's form list.
#[derive(Debug, Clone, Serialize)]
struct V2Analysis {
    entry: i32,
    steps: Vec<V2Step>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_form: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_reading: Option<String>,
    description: String,
    #[serde(skip_serializing_if = "is_true")]
    reading_matched: bool,
}

/// One conjugation step. `steps[0]` applies to the dictionary form; the last
/// step produces the surface form.
#[derive(Debug, Clone, Serialize)]
struct V2Step {
    form: String,
    pos: String,
    #[serde(skip_serializing_if = "is_false")]
    negative: bool,
    #[serde(skip_serializing_if = "is_false")]
    formal: bool,
}

/// The suffix grammar that matched a compound member: machine `class` plus
/// the display description.
#[derive(Debug, Clone, Serialize)]
struct V2Suffix {
    #[serde(skip_serializing_if = "Option::is_none")]
    class: Option<String>,
    description: String,
}

/// Number+counter info, with v1's `"Value: "` prefix stripped.
#[derive(Debug, Clone, Serialize)]
struct V2Counter {
    value: String,
    #[serde(skip_serializing_if = "is_false")]
    ordinal: bool,
}

/// A dictionary analysis tied at the same span as the winning token — a slim
/// reference, not a nested token. Carries its own score, not the winner's.
#[derive(Debug, Clone, Serialize)]
struct V2Alternative {
    #[serde(skip_serializing_if = "Option::is_none")]
    entry: Option<i32>,
    score: i32,
    reading: String,
    romanization: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    conjugation: Vec<V2Analysis>,
}

/// One dictionary entry: its writings, readings, and senses, in JMdict order.
#[derive(Debug, Clone, Serialize)]
struct V2Entry {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    kanji: Vec<V2Form>,
    kana: Vec<V2Form>,
    senses: Vec<V2Sense>,
}

/// One writing or reading of an entry. `common` is JMdict priority data:
/// `0` = common but unranked, `1`–`48` = newspaper frequency bucket.
#[derive(Debug, Clone, Serialize)]
struct V2Form {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    common: Option<i32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    furigana: Vec<V2Furigana>,
}

/// One sense. `restrict_kanji`/`restrict_kana` name the forms the sense
/// applies to (JMdict `stagk`/`stagr`); absent = all forms.
#[derive(Debug, Clone, Serialize)]
struct V2Sense {
    pos: Vec<String>,
    gloss: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    misc: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    field: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dial: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    restrict_kanji: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    restrict_kana: Vec<String>,
}

fn is_false(flag: &bool) -> bool {
    !*flag
}

fn is_true(flag: &bool) -> bool {
    *flag
}

/// Build the v2 document from a `romanize*` result.
///
/// `text` is the original input. The top-level `tokens`/`score`/
/// `romanization` come from the best path (beam rank 0); with
/// `include_paths` and a genuinely ambiguous input, every kept reading is
/// rendered into `paths`. Entry ids are collected across all rendered paths
/// and resolved once into `entries` (Full only).
fn to_v2<P>(
    ctx: &KaniranContext,
    text: &str,
    segments: &[RomanizeStarSegment<P>],
    method: KaniRomanizeMethod<'_>,
    options: V2Options,
) -> Result<V2Document, KaniDbError> {
    // Context choice mirrors `romanize_star_`'s own normalize call
    // (core/romanize.rs) — keep in sync, or spans desync from segments.
    let context = match method {
        KaniRomanizeMethod::Kana => NormalizationContext::Kana,
        KaniRomanizeMethod::Method(_) => NormalizationContext::Default,
    };
    let (_normalized, spans) = kani_normalize_spanned(text, context);
    let original_chars: Vec<char> = text.chars().collect();

    let builder = V2Builder {
        ctx,
        method,
        include_furigana: options.include_furigana,
        spans: &spans,
        original_chars: &original_chars,
    };

    // Beam ranks available = the most alternatives any one word segment kept.
    let n_paths = segments
        .iter()
        .map(|segment| match segment {
            RomanizeStarSegment::Word(alternatives) => alternatives.len(),
            RomanizeStarSegment::Misc(_) => 0,
        })
        .max()
        .unwrap_or(0);

    let mut entry_seqs: BTreeSet<i32> = BTreeSet::new();

    let best = build_path(&builder, segments, 0, &mut entry_seqs)?;

    let paths = if options.include_paths && n_paths > 1 {
        let mut paths = Vec::with_capacity(n_paths);
        paths.push(best.clone());
        for rank in 1..n_paths {
            paths.push(build_path(&builder, segments, rank, &mut entry_seqs)?);
        }
        Some(paths)
    } else {
        None
    };

    let entries = if options.include_entries {
        Some(build_entries(ctx, &entry_seqs, options.include_furigana)?)
    } else {
        None
    };

    Ok(V2Document {
        text: text.to_owned(),
        romanization: best.romanization,
        score: best.score,
        tokens: best.tokens,
        entries,
        paths,
    })
}

/// Render one beam path (rank `rank`) across all segments into a flat token
/// list. Each word segment contributes its `rank`-th alternative, clamped to
/// its last when it kept fewer. `compound` ids restart per path.
fn build_path<P>(
    builder: &V2Builder<'_>,
    segments: &[RomanizeStarSegment<P>],
    rank: usize,
    entry_seqs: &mut BTreeSet<i32>,
) -> Result<V2Path, KaniDbError> {
    let mut tokens = Vec::new();
    let mut romaji_parts: Vec<String> = Vec::new();
    let mut base = 0usize; // running char offset into the normalized stream
    let mut score_total = 0i32;
    let mut compound_counter = 0u32;

    for segment in segments {
        match segment {
            RomanizeStarSegment::Misc(misc) => {
                let len = misc.chars().count();
                tokens.push(builder.gap_token(misc, base, len));
                romaji_parts.push(misc.clone());
                base += len;
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
                        &mut compound_counter,
                        &mut tokens,
                        entry_seqs,
                    )?;
                }
                base += chunk_len;
            }
        }
    }

    // Every path must account for the whole normalized stream, or the
    // verbatim slices above indexed the wrong input chars.
    assert_eq!(
        base,
        builder.spans.normalized_len(),
        "v2 token spans desynced from the normalized input"
    );

    Ok(V2Path {
        score: score_total,
        romanization: join_parts(&romaji_parts),
        tokens,
    })
}

/// The constants threaded through every token.
struct V2Builder<'a> {
    ctx: &'a KaniranContext,
    method: KaniRomanizeMethod<'a>,
    include_furigana: bool,
    spans: &'a KaniNormalizedSpans,
    original_chars: &'a [char],
}

/// What a token inherits from its place in the segmentation: its normalized
/// char span, compound group, suffix flag, and score. Tied analyses of one
/// span share a placement.
#[derive(Debug, Clone, Copy)]
struct TokenPlacement {
    start: usize,
    end: usize,
    compound: Option<u32>,
    is_suffix: bool,
    score: i32,
}

impl V2Builder<'_> {
    /// The verbatim input slice behind the normalized char range
    /// `[start, end)`.
    fn original_slice(&self, start: usize, end: usize) -> String {
        let norm_len = self.spans.normalized_len();
        let (from, to) = self.spans.source_range(start.min(norm_len), end.min(norm_len));
        self.original_chars[from..to].iter().collect()
    }

    /// Emit a top-level word as one or more tokens: a compound explodes into
    /// adjacent members sharing a fresh `compound` id; everything else is one
    /// token.
    fn explode_word(
        &self,
        word_info: &WordInfo,
        romanized: &str,
        base: usize,
        compound_counter: &mut u32,
        out: &mut Vec<V2Token>,
        entry_seqs: &mut BTreeSet<i32>,
    ) -> Result<(), KaniDbError> {
        let score = word_info.score.unwrap_or(0);
        // An alternative wrapper carries components but is one word at one
        // span, not a compound; route it to `build_token`, which promotes the
        // best entry and fills `alternatives`.
        if word_info.components.is_empty() || word_info.alternative {
            let placement = TokenPlacement {
                start: base + word_info.start.unwrap_or(0),
                end: base + word_info.end.unwrap_or(0),
                compound: None,
                is_suffix: false,
                score,
            };
            out.push(self.build_token(word_info, romanized.to_owned(), placement, entry_seqs)?);
            return Ok(());
        }

        // Members carry the compound's score (v1 gives them none of their
        // own). They carry no offsets of their own either, so distribute the
        // compound's span across them by each member's normalized length.
        *compound_counter += 1;
        let id = *compound_counter;
        let mut member_start = base + word_info.start.unwrap_or(0);
        for member in &word_info.components {
            let member_end = member_start + member.text.chars().count();
            let placement = TokenPlacement {
                start: member_start,
                end: member_end,
                compound: Some(id),
                is_suffix: !member.primary,
                score,
            };
            let member_romaji = romanize_word_info(member, self.method);
            out.push(self.build_token(member, member_romaji, placement, entry_seqs)?);
            member_start = member_end;
        }
        Ok(())
    }

    /// Build one token from a single `WordInfo`.
    fn build_token(
        &self,
        word_info: &WordInfo,
        romanization: String,
        placement: TokenPlacement,
        entry_seqs: &mut BTreeSet<i32>,
    ) -> Result<V2Token, KaniDbError> {
        // A tied-analyses wrapper: promote the best entry into the token
        // itself and reference the rest as slim alternatives. All tied
        // entries share the span.
        if word_info.alternative && !word_info.components.is_empty() {
            let (best, rest) = word_info.components.split_first().expect("non-empty");
            let mut token =
                self.build_token(best, romanize_word_info(best, self.method), placement, entry_seqs)?;
            token.alternatives = rest
                .iter()
                .map(|tied| self.build_alternative(tied, entry_seqs))
                .collect::<Result<_, _>>()?;
            return Ok(token);
        }

        let reading = first_reading(word_info);
        let analyses = self.analyze(word_info, entry_seqs)?;
        let entry = analyses
            .first()
            .map(|analysis| analysis.entry)
            .or_else(|| single_seq(&word_info.seq));
        if let Some(entry) = entry {
            entry_seqs.insert(entry);
        }

        let suffix = if placement.is_suffix {
            entry.and_then(|seq| self.suffix_info(seq))
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

        let text = self.original_slice(placement.start, placement.end);

        // Ruby alignment runs on the normalized word text; when the verbatim
        // slice differs (exotic input), skip rather than emit segments that
        // don't concatenate to `text`. Entry-less tokens (raw-character
        // fallback paths) skip too — with no dictionary reading, the aligner
        // would only "match" the kanji against itself as an irregular.
        let furigana =
            if self.include_furigana && entry.is_some() && text == word_info.text {
                self.align_furigana(&word_info.text, &reading)?
            } else {
                Vec::new()
            };

        Ok(V2Token {
            text,
            reading: Some(reading),
            romanization,
            furigana,
            entry,
            score: Some(placement.score),
            conjugation: analyses,
            compound: placement.compound,
            suffix,
            counter,
            alternatives: Vec::new(),
            gap: false,
        })
    }

    /// A gap token: text with no dictionary match (punctuation, latin,
    /// unrecognized spans). `romanization` is the normalized/romanized form;
    /// `text` stays verbatim.
    fn gap_token(&self, romanization: &str, base: usize, len: usize) -> V2Token {
        V2Token {
            text: self.original_slice(base, base + len),
            romanization: romanization.to_owned(),
            gap: true,
            ..Default::default()
        }
    }

    /// A tied analysis as a slim entry reference with its own score.
    fn build_alternative(
        &self,
        word_info: &WordInfo,
        entry_seqs: &mut BTreeSet<i32>,
    ) -> Result<V2Alternative, KaniDbError> {
        let analyses = self.analyze(word_info, entry_seqs)?;
        let entry = analyses
            .first()
            .map(|analysis| analysis.entry)
            .or_else(|| single_seq(&word_info.seq));
        if let Some(entry) = entry {
            entry_seqs.insert(entry);
        }
        Ok(V2Alternative {
            entry,
            score: word_info.score.unwrap_or(0),
            reading: first_reading(word_info),
            romanization: romanize_word_info(word_info, self.method),
            conjugation: analyses,
        })
    }

    /// Resolve a word's conjugation analyses, empty if it is a dictionary
    /// form.
    fn analyze(
        &self,
        word_info: &WordInfo,
        entry_seqs: &mut BTreeSet<i32>,
    ) -> Result<Vec<V2Analysis>, KaniDbError> {
        let Some(seq) = single_seq(&word_info.seq) else {
            return Ok(Vec::new());
        };
        let text = match &word_info.true_text {
            Some(true_text) => FilterPropsText::One(true_text),
            None => FilterPropsText::None,
        };
        self.flatten_conj(seq, word_info.conjugations.as_ref(), text, entry_seqs)
    }

    /// Walk the conjugation rows for `seq`, flattening each via chain into
    /// one analysis whose `steps` run root-to-surface.
    ///
    /// Mirrors the data access of `conj_info_json_star_`: the base level
    /// (`seq_via == None`) holds the root `seq_from`; a `via` level recurses
    /// and appends its own steps after the deeper ones.
    fn flatten_conj(
        &self,
        seq: i32,
        conjugations: Option<&WordConjugations>,
        text: FilterPropsText<'_>,
        entry_seqs: &mut BTreeSet<i32>,
    ) -> Result<Vec<V2Analysis>, KaniDbError> {
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
                    entry_seqs.insert(conj.seq_from);
                    let description = build_display(&surface_steps);
                    out.push(V2Analysis {
                        entry: conj.seq_from,
                        steps: surface_steps,
                        base_form,
                        base_reading,
                        description,
                        reading_matched: true,
                    });
                }
                Some(via) => {
                    let deeper =
                        self.flatten_conj(via, None, FilterPropsText::None, entry_seqs)?;
                    for mut chain in deeper {
                        chain.steps.extend(surface_steps.iter().cloned());
                        chain.description = build_display(&chain.steps);
                        out.push(chain);
                    }
                    via_used.push(via);
                }
            }
        }

        Ok(out)
    }

    /// The suffix-grammar annotation for a suffix member's entry: machine
    /// class plus display description.
    fn suffix_info(&self, seq: i32) -> Option<V2Suffix> {
        let description = get_suffix_description(self.ctx, seq)?;
        let class = suffix_class(self.ctx).get(&seq).cloned();
        Some(V2Suffix {
            class,
            description: description.to_owned(),
        })
    }

    /// Ruby segments for `text` against `reading` via the ported kanji
    /// reading alignment; empty when there is nothing to align or no
    /// alignment exists.
    fn align_furigana(
        &self,
        text: &str,
        reading: &str,
    ) -> Result<Vec<V2Furigana>, KaniDbError> {
        if !text.chars().any(is_kanji_char) {
            return Ok(Vec::new());
        }
        let Some(segments) = match_readings(self.ctx, text, reading)? else {
            return Ok(Vec::new());
        };
        Ok(segments.into_iter().map(furigana_from_segment).collect())
    }
}

fn furigana_from_segment(segment: MatchedSegment) -> V2Furigana {
    match segment {
        MatchedSegment::Kanji { kanji, reading } => V2Furigana {
            text: kanji,
            reading: Some(reading.reading),
        },
        MatchedSegment::NonKanji(text) => V2Furigana {
            text,
            reading: None,
        },
    }
}

/// `*kanji-regex*` membership — same range as `kanji::matching::is_kanji`
/// (private there; keep in sync).
fn is_kanji_char(c: char) -> bool {
    matches!(c, '々' | 'ヶ' | '〆' | '\u{4e00}'..='\u{9faf}')
}

// --- entries table ---

/// The sense_prop tags the entries table surfaces. Superset of the ported
/// senses layer's TAGS (`dict/senses.rs`): adds `misc` and `dial`.
const ENTRY_SENSE_TAGS: &[&str] = &["pos", "s_inf", "stagk", "stagr", "field", "misc", "dial"];

/// Resolve every referenced entry id into a `V2Entry`: forms with
/// commonness, structured senses, and (with `include_furigana`) headword
/// furigana on kanji forms.
fn build_entries(
    ctx: &KaniranContext,
    entry_seqs: &BTreeSet<i32>,
    include_furigana: bool,
) -> Result<BTreeMap<i32, V2Entry>, KaniDbError> {
    let seq_list: Vec<i32> = entry_seqs.iter().copied().collect();

    let mut kanji_rows = ctx.store.kanji_texts_by_seq_any(&seq_list)?;
    kanji_rows.sort_by_key(|row| (row.seq, row.ord));
    let mut kana_rows = ctx.store.kana_texts_by_seq_any(&seq_list)?;
    kana_rows.sort_by_key(|row| (row.seq, row.ord));

    let mut entries: BTreeMap<i32, V2Entry> = BTreeMap::new();
    for &seq in &seq_list {
        let entry_kana: Vec<&KanaText> =
            kana_rows.iter().filter(|row| row.seq == seq).collect();

        let kana: Vec<V2Form> = entry_kana
            .iter()
            .map(|row| V2Form {
                text: row.text.clone().into_owned(),
                common: row.common,
                tags: parse_common_tags(&row.common_tags),
                furigana: Vec::new(),
            })
            .collect();

        let entry_kanji: Vec<_> = kanji_rows.iter().filter(|row| row.seq == seq).collect();
        let restricted: Vec<(String, String)> = if entry_kanji.is_empty() {
            Vec::new()
        } else {
            ctx.store
                .restricted_readings_by_seq(seq)?
                .into_iter()
                .map(|(reading, text)| (reading.into_owned(), text.into_owned()))
                .collect()
        };

        let mut kanji: Vec<V2Form> = Vec::with_capacity(entry_kanji.len());
        for row in entry_kanji {
            let paired_reading = include_furigana
                .then(|| reading_for_kanji_form(&row.text, &entry_kana, &restricted))
                .flatten();
            let furigana = match paired_reading {
                Some(reading) => {
                    match match_readings(ctx, &row.text, reading)? {
                        Some(segments) => {
                            segments.into_iter().map(furigana_from_segment).collect()
                        }
                        None => Vec::new(),
                    }
                }
                None => Vec::new(),
            };
            kanji.push(V2Form {
                text: row.text.clone().into_owned(),
                common: row.common,
                tags: parse_common_tags(&row.common_tags),
                furigana,
            });
        }

        entries.insert(
            seq,
            V2Entry {
                kanji,
                kana,
                senses: fetch_senses(ctx, seq)?,
            },
        );
    }
    Ok(entries)
}

/// The first kana form a kanji writing pairs with: skips `nokanji` readings
/// and honors restricted-reading rows; falls back to the entry's first kana.
fn reading_for_kanji_form<'a>(
    kanji_text: &str,
    entry_kana: &[&'a KanaText],
    restricted: &[(String, String)],
) -> Option<&'a str> {
    entry_kana
        .iter()
        .filter(|kana| !kana.nokanji)
        .find(|kana| {
            let mut restrictions = restricted
                .iter()
                .filter(|(reading, _)| reading == kana.text.as_ref())
                .peekable();
            restrictions.peek().is_none()
                || restrictions.any(|(_, restricted_kanji)| restricted_kanji == kanji_text)
        })
        .or_else(|| entry_kana.first())
        .map(|kana| kana.text.as_ref())
}

/// Structured senses for one entry, straight off the gloss and sense_prop
/// rows. Diverges from the ported senses layer deliberately: arrays instead
/// of joined strings, `misc`/`dial` included, restrictions exposed instead
/// of pre-filtered, and prop groups in row order (no final-group reversal).
fn fetch_senses(ctx: &KaniranContext, seq: i32) -> Result<Vec<V2Sense>, KaniDbError> {
    let gloss_rows = ctx.store.sense_gloss_rows(seq)?;
    let prop_rows = ctx.store.sense_prop_rows_tagged(seq, ENTRY_SENSE_TAGS)?;

    let mut senses: Vec<(i32, V2Sense)> = gloss_rows
        .into_iter()
        .map(|(ord, gloss)| {
            (
                ord,
                V2Sense {
                    pos: Vec::new(),
                    gloss: split_glosses(gloss.as_deref().unwrap_or_default()),
                    info: None,
                    misc: Vec::new(),
                    field: Vec::new(),
                    dial: Vec::new(),
                    restrict_kanji: Vec::new(),
                    restrict_kana: Vec::new(),
                },
            )
        })
        .collect();

    for (sense_ord, tag, text) in prop_rows {
        let Some((_, sense)) = senses.iter_mut().find(|(ord, _)| *ord == sense_ord) else {
            continue;
        };
        let text = text.into_owned();
        match tag.as_ref() {
            "pos" => sense.pos.push(text),
            "s_inf" => match &mut sense.info {
                Some(info) => {
                    info.push_str("; ");
                    info.push_str(&text);
                }
                None => sense.info = Some(text),
            },
            "misc" => sense.misc.push(text),
            "field" => sense.field.push(text),
            "dial" => sense.dial.push(text),
            "stagk" => sense.restrict_kanji.push(text),
            "stagr" => sense.restrict_kana.push(text),
            _ => {}
        }
    }

    // JMdict pos carry-forward: a sense without pos props inherits the
    // previous sense's (same rule as the ported get-senses-json rpos walk).
    let mut prev_pos: Vec<String> = Vec::new();
    for (_, sense) in &mut senses {
        if sense.pos.is_empty() {
            sense.pos = prev_pos.clone();
        } else {
            prev_pos = sense.pos.clone();
        }
    }

    Ok(senses.into_iter().map(|(_, sense)| sense).collect())
}

// --- shared helpers ---

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

/// First kana reading of a word, marks stripped.
fn first_reading(word_info: &WordInfo) -> String {
    match &word_info.kana {
        None => strip_marks(&word_info.text),
        Some(WordInfoKana::Single(kana)) => strip_marks(kana),
        Some(WordInfoKana::Multi(list)) => list
            .iter()
            .flatten()
            .find_map(first_kana)
            .map(|kana| strip_marks(&kana))
            .unwrap_or_else(|| strip_marks(&word_info.text)),
    }
}

fn first_kana(kana: &WordInfoKana) -> Option<String> {
    match kana {
        WordInfoKana::Single(text) => Some(text.clone()),
        WordInfoKana::Multi(list) => list.iter().flatten().find_map(first_kana),
    }
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

fn strip_value_prefix(value: &str) -> String {
    value.strip_prefix("Value: ").unwrap_or(value).to_owned()
}

/// `"[ichi1][news2][nf25]"` → `["ichi1", "news2", "nf25"]`; empty → empty.
fn parse_common_tags(raw: &str) -> Vec<String> {
    raw.split(['[', ']'])
        .filter(|tag| !tag.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Invert the `"; "` gloss join into one string per gloss.
fn split_glosses(joined: &str) -> Vec<String> {
    if joined.is_empty() {
        return Vec::new();
    }
    joined.split("; ").map(str::to_owned).collect()
}

#[cfg(test)]
mod tests;
