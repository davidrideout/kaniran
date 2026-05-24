//! Manual fixture-replay runner for `ICHIRAN/DICT:JOIN-SUBSTRING-WORDS`.
//! Source under test: `src/dict/join_substring_words.rs`.
//!
//! Run with:
//!   cargo run --release --bin join_substring_words_test -- \
//!       --path corpus/substring_2026_05_14/dict/join_substring_words.parquet
//!
//! Args shape: `[<str>]`.
//!
//! Result shape (one captured value): a list of SEGMENT-LIST objects,
//! each `{"_meta":{"class":"SEGMENT-LIST"}, "start", "end", "matches",
//! "segments":[{SEGMENT}, ...]}`. Unlike `join-substring-words*`, the
//! segments here are gen-scored: every SEGMENT carries a populated
//! `score` and `info` plist, both compared in full.
//!
//! ## id-stripping
//!
//! Same asymmetry as `join_substring_words_star_test` /
//! `find_word_full_test`: the inner `find-word` reads the
//! `*substring-hash*` `make-instance` branch (`dict.lisp:493`), leaving
//! the `id` slot unbound — the projector emits `"id": null`. The Rust
//! port reads its `substring_hash` cache (rows from `SELECT *`, real
//! ids). Both sides are zeroed before comparison; `seq + text + ord`
//! identifies the row.
//!
//! ## comparison
//!
//! Segment-lists are ordered `(start, end)` ascending on both sides
//! (the upstream loop walks `result` in order), so they compare
//! positionally on `(start, end, matches)`. Segment order WITHIN a list
//! follows `cull-segments`' two stable sorts; ties among equal
//! `(common, score)` fall back on `find-word`'s unordered SQL, so the
//! per-list segments are compared as a sorted multiset of full-field
//! fingerprints. The fingerprint covers every compared slot — `start`,
//! `end`, `score`, `text`, `word` (deep `Debug` after id-stripping),
//! and the six-key `info` plist (with `posi` sorted, since
//! `get-non-arch-posi`'s SQL is unordered). `ConjData` / `ConjProp`
//! `Debug` covers exactly the fields the `gen-score` audit compares
//! field-by-field, so the fingerprint is a complete check, not a
//! lossy one.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::compound_text_class::CompoundText;
use kaniran_core::dict::counter_text_class::{Counter, CounterSource, CounterText};
use kaniran_core::dict::join_substring_words::join_substring_words;
use kaniran_core::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use kaniran_core::dict::proxy_text_class::ProxyText;
use kaniran_core::dict::segment_list_struct::SegmentList;
use kaniran_core::dict::segment_struct::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};

use common::{
    captured_class, parse_captured_word, parse_conj_list, parse_int_list, parse_kpcl,
    parse_opt_i32, parse_score_info, parse_string_list, single_result, CapturedRow,
};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:JOIN-SUBSTRING-WORDS";

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.is_empty() {
        return Err("join-substring-words args empty".into());
    }
    let str = row.args[0]
        .as_str()
        .ok_or_else(|| format!("arg 0 (str) not string: {}", row.args[0]))?;

    let mut actual = join_substring_words(ctx, str)
        .await
        .map_err(|err| format!("join_substring_words query: {}", err))?;
    for sl in actual.iter_mut() {
        for segment in sl.segments.iter_mut() {
            strip_ids_word(&mut segment.word);
        }
    }

    let expected_value = single_result(&row.result)?;
    let mut expected = parse_expected_segment_lists(expected_value)?;
    for sl in expected.iter_mut() {
        for segment in sl.segments.iter_mut() {
            strip_ids_word(&mut segment.word);
        }
    }

    if actual.len() != expected.len() {
        return Err(format!(
            "segment-list count: rust={} lisp={} (rust spans: {:?}, lisp spans: {:?})",
            actual.len(),
            expected.len(),
            actual.iter().map(|sl| (sl.start, sl.end)).collect::<Vec<_>>(),
            expected.iter().map(|sl| (sl.start, sl.end)).collect::<Vec<_>>(),
        ));
    }
    for (index, (actual_sl, expected_sl)) in actual.iter().zip(expected.iter()).enumerate() {
        compare_segment_list(index, actual_sl, expected_sl)?;
    }
    Ok(())
}

fn compare_segment_list(
    index: usize,
    actual: &SegmentList,
    expected: &SegmentList,
) -> Result<(), String> {
    if actual.start != expected.start || actual.end != expected.end {
        return Err(format!(
            "segment-list {} span: rust=({}, {}) lisp=({}, {})",
            index, actual.start, actual.end, expected.start, expected.end,
        ));
    }
    if actual.matches != expected.matches {
        return Err(format!(
            "segment-list {} (span {},{}) matches: rust={} lisp={}",
            index, actual.start, actual.end, actual.matches, expected.matches,
        ));
    }
    let actual_fp = sorted_fingerprints(&actual.segments);
    let expected_fp = sorted_fingerprints(&expected.segments);
    if actual_fp != expected_fp {
        return Err(format!(
            "segment-list {} (span {},{}) segments differ:\n  rust = {:#?}\n  lisp = {:#?}",
            index, actual.start, actual.end, actual_fp, expected_fp,
        ));
    }
    Ok(())
}

fn sorted_fingerprints(segments: &[Segment]) -> Vec<String> {
    let mut fps: Vec<String> = segments.iter().map(segment_fingerprint).collect();
    fps.sort();
    fps
}

fn segment_fingerprint(seg: &Segment) -> String {
    format!(
        "start={} end={} score={:?} text={:?} info={} word={:?}",
        seg.start,
        seg.end,
        seg.score,
        seg.text,
        info_fingerprint(seg.info.as_ref()),
        seg.word,
    )
}

fn info_fingerprint(info: Option<&KaniSegmentInfo>) -> String {
    match info {
        None => "None".to_string(),
        Some(info) => {
            // posi is order-insensitive (get-non-arch-posi SQL is unordered).
            let mut posi = info.posi.clone();
            posi.sort();
            format!(
                "(posi={:?} seq_set={:?} conj={:?} common={:?} score_info={:?} kpcl={:?})",
                posi, info.seq_set, info.conj, info.common, info.score_info, info.kpcl,
            )
        }
    }
}

// --- expected parsing ------------------------------------------------------

fn parse_expected_segment_lists(value: &Value) -> Result<Vec<SegmentList>, String> {
    let arr = match value {
        Value::Null => return Ok(Vec::new()),
        Value::Array(arr) => arr,
        other => return Err(format!("expected segment-list array or null, got {}", other)),
    };
    arr.iter()
        .enumerate()
        .map(|(index, sl)| {
            parse_expected_segment_list(sl).map_err(|err| format!("segment-list {}: {}", index, err))
        })
        .collect()
}

fn parse_expected_segment_list(value: &Value) -> Result<SegmentList, String> {
    let class = captured_class(value)?;
    if class != "SEGMENT-LIST" {
        return Err(format!("expected SEGMENT-LIST class, got :{}", class));
    }
    if let Some(obj) = value.as_object() {
        for key in obj.keys() {
            match key.as_str() {
                "_meta" | "segments" | "start" | "end" | "matches" | "top" => {}
                other => {
                    return Err(format!(
                        "unhandled segment-list slot: {} (audit drift)",
                        other
                    ))
                }
            }
        }
    }
    if let Some(top) = value.get("top") {
        if !top.is_null() {
            return Err(format!(
                "top: lisp captured non-null ({}) but join-substring-words leaves top nil",
                top
            ));
        }
    }
    let start = get_usize(value, "start")?;
    let end = get_usize(value, "end")?;
    let matches = get_usize(value, "matches")?;
    let segments_value = value
        .get("segments")
        .ok_or_else(|| "missing segments".to_string())?;
    let segments_arr = segments_value
        .as_array()
        .ok_or_else(|| format!("segments: expected array, got {}", segments_value))?;
    let mut segments = Vec::with_capacity(segments_arr.len());
    for (seg_index, seg) in segments_arr.iter().enumerate() {
        segments.push(
            parse_expected_segment(seg).map_err(|err| format!("segment {}: {}", seg_index, err))?,
        );
    }
    Ok(SegmentList {
        segments,
        start,
        end,
        top: None,
        matches,
    })
}

fn parse_expected_segment(value: &Value) -> Result<Segment, String> {
    let class = captured_class(value)?;
    if class != "SEGMENT" {
        return Err(format!("expected SEGMENT class, got :{}", class));
    }
    if let Some(obj) = value.as_object() {
        for key in obj.keys() {
            match key.as_str() {
                "_meta" | "start" | "end" | "word" | "score" | "info" | "text" | "top" => {}
                other => return Err(format!("unhandled segment slot: {} (audit drift)", other)),
            }
        }
    }
    if let Some(top) = value.get("top") {
        if !top.is_null() {
            return Err(format!("segment top: lisp captured non-null ({})", top));
        }
    }
    let start = get_usize(value, "start")?;
    let end = get_usize(value, "end")?;
    let word = parse_captured_word(value.get("word").ok_or_else(|| "missing word".to_string())?)?;
    let score = match value.get("score").unwrap_or(&Value::Null) {
        Value::Null => None,
        Value::Number(n) => {
            Some(n.as_i64().ok_or_else(|| format!("score not i64: {}", n))? as i32)
        }
        other => return Err(format!("score: expected number/null, got {}", other)),
    };
    let info = match value.get("info").unwrap_or(&Value::Null) {
        Value::Null => None,
        other => Some(parse_info_plist(other)?),
    };
    let text = match value.get("text").unwrap_or(&Value::Null) {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        other => return Err(format!("text: expected string/null, got {}", other)),
    };
    Ok(Segment {
        start,
        end,
        word,
        score,
        info,
        top: None,
        text,
    })
}

fn get_usize(value: &Value, key: &str) -> Result<usize, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .ok_or_else(|| format!("{}: not a usize ({:?})", key, value.get(key)))
}

/// Parse the captured `info` plist into [`KaniSegmentInfo`]. Same shape
/// as the `gen-score` audit; kept inline per the audit-binary
/// independence convention.
fn parse_info_plist(v: &Value) -> Result<KaniSegmentInfo, String> {
    let arr = v
        .as_array()
        .ok_or_else(|| format!("info: expected plist array, got {}", v))?;
    if arr.len() % 2 != 0 {
        return Err(format!("info: odd plist length {}", arr.len()));
    }
    let mut posi: Vec<String> = Vec::new();
    let mut seq_set: Vec<i32> = Vec::new();
    let mut conj = Vec::new();
    let mut common: Option<i32> = None;
    let mut score_info = KaniScoreInfo {
        prop_score: 0,
        kanji_break: Vec::new(),
        use_length_bonus: 0,
        split_info: KaniSplitInfo::None,
    };
    let mut kpcl = (false, false, false, false);
    let mut i = 0;
    while i < arr.len() {
        let key = arr[i]
            .as_str()
            .ok_or_else(|| format!("info key at {} not string: {}", i, arr[i]))?;
        let val = &arr[i + 1];
        match key {
            ":POSI" => posi = parse_string_list(val, "posi")?,
            ":SEQ-SET" => seq_set = parse_int_list(val, "seq-set")?,
            ":CONJ" => conj = parse_conj_list(val)?,
            ":COMMON" => common = parse_opt_i32(val, "common")?,
            ":SCORE-INFO" => score_info = parse_score_info(val)?,
            ":KPCL" => kpcl = parse_kpcl(val)?,
            other => return Err(format!("info: unknown key {}", other)),
        }
        i += 2;
    }
    Ok(KaniSegmentInfo {
        posi,
        seq_set,
        conj,
        common,
        score_info,
        kpcl,
    })
}

// --- id-stripping (mirrors join_substring_words_star_test) ----------------

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
