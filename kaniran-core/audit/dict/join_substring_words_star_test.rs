//! Manual fixture-replay runner for `ICHIRAN/DICT:JOIN-SUBSTRING-WORDS*`.
//! Source under test: `src/dict/join_substring_words_star_.rs`.
//!
//! Run with:
//!   cargo run --release --bin join_substring_words_star_test -- \
//!       --path corpus/substring_2026_05_14/dict/join_substring_words_star_.parquet
//!
//! Args shape: `[<str>]`.
//!
//! Result shape (two captured values):
//!   result[0] — list of `[start, end, [segment, ...]]` triples, each
//!     segment `{"_meta":{"class":"SEGMENT"}, "start", "end", "word", ...}`.
//!   result[1] — the kanji-break list (`[int, ...]` or null).
//!
//! ## id-stripping
//!
//! Under `join-substring-words*` the inner `find-word` reads the
//! `*substring-hash*` `make-instance` branch (`dict.lisp:493`), which
//! leaves the `id` slot unbound — the projector emits `"id": null`. The
//! Rust port reads its `substring_hash` cache (rows from `SELECT *`,
//! real ids). Both sides are zeroed before comparison; `seq + text +
//! ord` already identifies the row. Same asymmetry handled by
//! `find_word_full_test`.
//!
//! ## comparison
//!
//! The outer triple order is `(start, end)` ascending on both sides
//! (the upstream nested loop), so triples compare positionally. Segment
//! order within a triple follows `find-word`'s unordered SQL, so the
//! per-triple segment word fingerprints are sorted before comparison.
//! The kanji-break list is deterministic and compared in order.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::compound_text_class::CompoundText;
use kaniran_core::dict::counter_text_class::{Counter, CounterSource, CounterText};
use kaniran_core::dict::join_substring_words_star_::join_substring_words_star_;
use kaniran_core::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use kaniran_core::dict::proxy_text_class::ProxyText;
use kaniran_core::dict::segment_struct::Segment;

use common::{parse_captured_word, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:JOIN-SUBSTRING-WORDS*";

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.is_empty() {
        return Err("join-substring-words* args empty".into());
    }
    let str = row.args[0]
        .as_str()
        .ok_or_else(|| format!("arg 0 (str) not string: {}", row.args[0]))?;

    let (mut result, kanji_break) = join_substring_words_star_(ctx, str)
        .await
        .map_err(|err| format!("join_substring_words_star_ query: {}", err))?;
    for (_, _, segments) in result.iter_mut() {
        for segment in segments.iter_mut() {
            strip_ids_word(&mut segment.word);
        }
    }

    // result[0] — the triples; result[1] — the kanji-break list.
    let expected_result = row
        .result
        .first()
        .ok_or_else(|| "expected at least 1 result value (triples), got 0".to_string())?;
    let expected_kanji_break = row
        .result
        .get(1)
        .ok_or_else(|| "expected 2nd result value (kanji-break), got 1".to_string())?;

    compare_kanji_break(&kanji_break, expected_kanji_break)?;
    compare_result(&result, expected_result)
}

fn compare_kanji_break(actual: &[usize], expected_value: &Value) -> Result<(), String> {
    let expected = parse_usize_list(expected_value)
        .map_err(|err| format!("kanji-break parse: {}", err))?;
    if actual != expected.as_slice() {
        return Err(format!(
            "kanji-break: rust={:?} lisp={:?}",
            actual, expected
        ));
    }
    Ok(())
}

fn compare_result(
    actual: &[(usize, usize, Vec<Segment>)],
    expected_value: &Value,
) -> Result<(), String> {
    let mut expected = parse_expected_triples(expected_value)?;
    for triple in expected.iter_mut() {
        for segment in triple.2.iter_mut() {
            strip_ids_word(&mut segment.word);
        }
    }
    if actual.len() != expected.len() {
        return Err(format!(
            "triple count: rust={} lisp={} (rust spans: {:?}, lisp spans: {:?})",
            actual.len(),
            expected.len(),
            actual.iter().map(|(s, e, _)| (*s, *e)).collect::<Vec<_>>(),
            expected.iter().map(|(s, e, _)| (*s, *e)).collect::<Vec<_>>(),
        ));
    }
    for (index, (actual_triple, expected_triple)) in
        actual.iter().zip(expected.iter()).enumerate()
    {
        if actual_triple.0 != expected_triple.0 || actual_triple.1 != expected_triple.1 {
            return Err(format!(
                "triple {} span: rust=({}, {}) lisp=({}, {})",
                index, actual_triple.0, actual_triple.1, expected_triple.0, expected_triple.1,
            ));
        }
        let actual_fp = sorted_actual_segment_fingerprints(&actual_triple.2);
        let expected_fp = sorted_expected_segment_fingerprints(&expected_triple.2);
        if actual_fp != expected_fp {
            return Err(format!(
                "triple {} (span {},{}) segments differ:\n  rust = {:?}\n  lisp = {:?}",
                index, actual_triple.0, actual_triple.1, actual_fp, expected_fp,
            ));
        }
    }
    Ok(())
}

/// Full-segment fingerprint of the Rust output: every [`Segment`] field
/// (`start`, `end`, `score`, `info` presence, `text`, and the `word`'s
/// complete `Debug`). `top` is omitted — it is unset at this stage on
/// both sides (the projector emits no `top` key; the port leaves it
/// `None`).
fn sorted_actual_segment_fingerprints(segments: &[Segment]) -> Vec<String> {
    let mut fps: Vec<String> = segments
        .iter()
        .map(|seg| {
            format!(
                "s={} e={} score={:?} info_absent={} text={:?} | {:?}",
                seg.start,
                seg.end,
                seg.score.map(i64::from),
                seg.info.is_none(),
                seg.text,
                seg.word,
            )
        })
        .collect();
    fps.sort();
    fps
}

fn sorted_expected_segment_fingerprints(segments: &[ExpectedSegment]) -> Vec<String> {
    let mut fps: Vec<String> = segments
        .iter()
        .map(|seg| {
            format!(
                "s={} e={} score={:?} info_absent={} text={:?} | {:?}",
                seg.start, seg.end, seg.score, seg.info_absent, seg.text, seg.word,
            )
        })
        .collect();
    fps.sort();
    fps
}

fn parse_usize_list(value: &Value) -> Result<Vec<usize>, String> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => arr
            .iter()
            .map(|item| {
                item.as_u64()
                    .map(|n| n as usize)
                    .ok_or_else(|| format!("not a usize: {}", item))
            })
            .collect(),
        other => Err(format!("expected array or null, got {}", other)),
    }
}

/// One captured SEGMENT: every field the projector emits. `start` /
/// `end` are the segment's own slots (equal the triple's by
/// `make-segment`); `score` / `info` / `text` are nil at this stage and
/// modeled as their absence so a non-nil capture would diverge.
struct ExpectedSegment {
    start: usize,
    end: usize,
    score: Option<i64>,
    info_absent: bool,
    text: Option<String>,
    word: KaniWordDispatchEnum,
}

/// Parse `result[0]` — a list of `[start, end, [segment, ...]]` triples.
/// Each segment is read in full: `start`, `end`, `score`, `info`,
/// `text`, and `word` (via [`parse_captured_word`]).
fn parse_expected_triples(
    value: &Value,
) -> Result<Vec<(usize, usize, Vec<ExpectedSegment>)>, String> {
    let triples = match value {
        Value::Null => return Ok(Vec::new()),
        Value::Array(arr) => arr,
        other => return Err(format!("result triples: expected array or null, got {}", other)),
    };
    let mut out = Vec::with_capacity(triples.len());
    for (index, triple) in triples.iter().enumerate() {
        let triple_arr = triple
            .as_array()
            .ok_or_else(|| format!("triple {} not array: {}", index, triple))?;
        if triple_arr.len() != 3 {
            return Err(format!(
                "triple {} expected [start, end, segments], got {} elements",
                index,
                triple_arr.len()
            ));
        }
        let start = triple_arr[0]
            .as_u64()
            .ok_or_else(|| format!("triple {} start not usize: {}", index, triple_arr[0]))?
            as usize;
        let end = triple_arr[1]
            .as_u64()
            .ok_or_else(|| format!("triple {} end not usize: {}", index, triple_arr[1]))?
            as usize;
        let segments_arr = triple_arr[2]
            .as_array()
            .ok_or_else(|| format!("triple {} segments not array: {}", index, triple_arr[2]))?;
        let mut segments = Vec::with_capacity(segments_arr.len());
        for (seg_index, segment) in segments_arr.iter().enumerate() {
            let ctx = || format!("triple {} segment {}", index, seg_index);
            let seg_start = segment
                .get("start")
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("{}: start not usize", ctx()))? as usize;
            let seg_end = segment
                .get("end")
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("{}: end not usize", ctx()))? as usize;
            let score = match segment.get("score") {
                None | Some(Value::Null) => None,
                Some(Value::Number(n)) => Some(
                    n.as_i64()
                        .ok_or_else(|| format!("{}: score not i64: {}", ctx(), n))?,
                ),
                Some(other) => return Err(format!("{}: score unexpected {}", ctx(), other)),
            };
            let info_absent = matches!(segment.get("info"), None | Some(Value::Null));
            let text = match segment.get("text") {
                None | Some(Value::Null) => None,
                Some(Value::String(s)) => Some(s.clone()),
                Some(other) => return Err(format!("{}: text unexpected {}", ctx(), other)),
            };
            let word_value = segment
                .get("word")
                .ok_or_else(|| format!("{}: missing word", ctx()))?;
            let word = parse_captured_word(word_value)
                .map_err(|err| format!("{}: {}", ctx(), err))?;
            segments.push(ExpectedSegment {
                start: seg_start,
                end: seg_end,
                score,
                info_absent,
                text,
                word,
            });
        }
        out.push((start, end, segments));
    }
    Ok(out)
}

// --- id-stripping (see module doc; mirrors find_word_full_test) -----------

fn strip_ids_word(word: &mut KaniWordDispatchEnum) {
    match word {
        KaniWordDispatchEnum::Kana(k) => k.id = 0,
        KaniWordDispatchEnum::Kanji(k) => k.id = 0,
        KaniWordDispatchEnum::Proxy(p) => strip_ids_proxy(p),
        KaniWordDispatchEnum::Compound(c) => strip_ids_compound(c),
        KaniWordDispatchEnum::Counter(c) => strip_ids_counter(c),
    }
}

fn strip_ids_simple(simple: &mut KaniSimpleTextDispatchEnum) {
    match simple {
        KaniSimpleTextDispatchEnum::Kana(k) => k.id = 0,
        KaniSimpleTextDispatchEnum::Kanji(k) => k.id = 0,
        KaniSimpleTextDispatchEnum::Proxy(p) => strip_ids_proxy(p),
    }
}

fn strip_ids_proxy(proxy: &mut ProxyText) {
    strip_ids_simple(&mut proxy.source);
}

fn strip_ids_compound(compound: &mut CompoundText) {
    strip_ids_word(&mut compound.primary);
    for w in compound.words.iter_mut() {
        strip_ids_word(w);
    }
    if let Some(sb) = compound.score_base.as_mut() {
        strip_ids_word(sb);
    }
}

fn strip_ids_counter(counter: &mut Counter) {
    let base: &mut CounterText = match counter {
        Counter::Base(b) => b,
        Counter::NumberText(c) => &mut c.0,
        Counter::Tsu(c) => &mut c.0,
        Counter::Age(c) => &mut c.0,
        Counter::DaysKun(c) => &mut c.0,
        Counter::DaysOn(c) => &mut c.0,
        Counter::Halfhour(c) => &mut c.0,
        Counter::Months(c) => &mut c.0,
        Counter::People(c) => &mut c.0,
        Counter::Wari(c) => &mut c.0,
        Counter::Hifumi(c) => &mut c.base,
    };
    if let Some(src) = base.source.as_mut() {
        match src {
            CounterSource::Kanji(k) => k.id = 0,
            CounterSource::Kana(k) => k.id = 0,
        }
    }
}

#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
