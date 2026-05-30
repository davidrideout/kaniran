//! Manual fixture-replay runner for `ICHIRAN/DICT:FIND-BEST-PATH`.
//! Source under test: `src/dict/find_best_path.rs`.
//!
//! Run with:
//!   cargo run --release --bin find_best_path_test -- \
//!       --path corpus/extracted_chunk_b_segmentation_2026_05_14/dict/find_best_path.parquet
//!
//! Captured shapes (`extracted_chunk_b_segmentation_2026_05_14` corpus):
//! - `args = [<segment-lists-or-nil>, <str-length>, ":LIMIT", <limit>]`.
//!   `args[0]` is a JSON array of SEGMENT-LIST objects (or null when
//!   the input list is empty). The tracer projects args POST-call
//!   (`trace_capture.lisp:179-194`); `find-best-path` mutates each
//!   segment-list via [`expand_segment_list`] (`segments`, `matches`)
//!   and sets/clears the per-list `top` (`dict.lisp:1196-1197`,
//!   `:1229-1230`), so `args[0][i].segments` is the post-expand sorted
//!   list and `args[0][i].matches` is the post-increment counter.
//! - `result = [<list-of-paths>]` — single-value return: a list of
//!   `(reversed-path . score)` cons cells, one per `top-array-item`.
//!   Each cons head is a list of SEGMENT-LIST / SYNERGY objects in
//!   left-to-right (post-reverse) order; the tail is the integer score.
//!   The all-gap seed (`dict.lisp:1193`) registers a nil payload —
//!   its cons head is `null` and its score is `gap-penalty 0 str-length`.
//!
//! Full-field round-trip (matches the convention from
//! `expand_segment_list_test` and `get_seg_splits_test`):
//! - SegmentList: `segments` (deep), `start`, `end`, `matches`.
//! - Segment: `start`, `end`, `score`, `text`, `word` (Debug-string
//!   compare), `info` (plist parsed into `KaniSegmentInfo`).
//! - Synergy: `description`, `connector`, `score`, `start`, `end`.
//! - `top` slots are transient and not asserted.
//!
//! Pre-call reconstruction (same approach as `expand_segment_list_test`,
//! kept inline per the audit-binary-independence convention): for each
//! captured post-state SegmentList, call `get_segsplit` on every
//! simple-text segment and identify which compound entry in the
//! post-state was inserted by the previous expand-segment-list run.
//! Filter those out to recover the pre-call segments list, then run
//! `find_best_path` and compare against the captured result.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::conj_data::ConjData;
use kaniran_core::dict::best_path::find_best_path;
use kaniran_core::dict::split::segsplit::get_segsplit;
use kaniran_core::dict::kani::KaniWordDispatchEnum;
use kaniran_core::dict::segment::SegmentList;
use kaniran_core::dict::segment::{
    KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment,
};
use kaniran_core::dict::grammar::synergy::Synergy;
use kaniran_core::dict::segment::PathElement;

use common::{
    captured_class, parse_captured_word, parse_conj_list, parse_int_list, parse_kpcl,
    parse_opt_i32, parse_score_info, parse_string_list, single_result, CapturedRow,
};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:FIND-BEST-PATH";

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    // Args: [segment-lists-or-nil, str-length, ":LIMIT", limit].
    if row.args.len() != 4 {
        return Err(format!(
            "expected 4 args (segment-lists, str-length, :LIMIT, limit), got {}",
            row.args.len()
        ));
    }
    let str_length = row.args[1]
        .as_i64()
        .ok_or_else(|| format!("arg 1 (str-length): not int: {}", row.args[1]))?
        as usize;
    let key = row.args[2]
        .as_str()
        .ok_or_else(|| format!("arg 2: expected \":LIMIT\" string, got {}", row.args[2]))?;
    if key != ":LIMIT" {
        return Err(format!("arg 2: expected :LIMIT keyword, got {}", key));
    }
    let limit = row.args[3]
        .as_i64()
        .ok_or_else(|| format!("arg 3 (limit): not int: {}", row.args[3]))?
        as usize;

    // arg 0: array of SegmentList JSON objects (post-call state) or null.
    let captured_post_state: Vec<SegmentList> = match &row.args[0] {
        Value::Null => Vec::new(),
        Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for (i, item) in arr.iter().enumerate() {
                out.push(
                    parse_segment_list_full(item)
                        .map_err(|err| format!("arg 0[{}]: {}", i, err))?,
                );
            }
            out
        }
        other => return Err(format!("arg 0: expected array/null, got {}", other)),
    };

    // Reconstruct pre-call state by filtering out segsplit-added compounds.
    let mut pre_state: Vec<SegmentList> = Vec::with_capacity(captured_post_state.len());
    for sl in &captured_post_state {
        let added_mask = mark_segsplit_added(ctx, &sl.segments).await?;
        let dropped = added_mask.iter().filter(|x| **x).count();
        let pre_matches = sl.matches.checked_sub(dropped).ok_or_else(|| {
            format!(
                "pre-call matches underflow: post.matches={} dropped={}",
                sl.matches, dropped
            )
        })?;
        let pre_segments: Vec<Segment> = sl
            .segments
            .iter()
            .zip(added_mask.iter())
            .filter_map(|(s, added)| if !*added { Some(s.clone()) } else { None })
            .collect();
        pre_state.push(SegmentList {
            segments: pre_segments,
            start: sl.start,
            end: sl.end,
            top: None,
            matches: pre_matches,
        });
    }

    let actual = find_best_path(ctx, &mut pre_state, str_length, Some(limit))
        .await
        .map_err(|err| format!("find_best_path: {}", err))?;

    // Result: [[<list-of (cons reversed-path score)>]].
    let result_value = single_result(&row.result)?;
    let captured_items: &[Value] = match result_value {
        Value::Null => &[],
        Value::Array(arr) => arr.as_slice(),
        other => return Err(format!("result[0]: expected array / null, got {}", other)),
    };
    if actual.len() != captured_items.len() {
        return Err(format!(
            "result len: rust={} lisp={}",
            actual.len(),
            captured_items.len()
        ));
    }
    for (i, (actual_pair, expected_cons)) in actual.iter().zip(captured_items.iter()).enumerate() {
        compare_path_score_pair(actual_pair, expected_cons)
            .map_err(|err| format!("result[{}]: {}", i, err))?;
    }
    Ok(())
}

// ----- pre-call reconstruction (mirror expand_segment_list_test) -----------

async fn mark_segsplit_added(
    ctx: &KaniranContext,
    segments: &[Segment],
) -> Result<Vec<bool>, String> {
    let mut added = vec![false; segments.len()];
    for (i, seg) in segments.iter().enumerate() {
        let candidate = match get_segsplit(ctx, seg).await {
            Ok(c) => c,
            Err(err) => return Err(format!("get_segsplit @ segments[{}]: {}", i, err)),
        };
        let Some(predicted) = candidate else { continue };
        let pred_kana = compound_kana(&predicted.word)?;
        let pred_primary_seq = compound_primary_seq(&predicted.word)?;
        let pred_text = predicted
            .text
            .as_deref()
            .ok_or_else(|| format!("predicted segsplit @ {} missing text", i))?;
        let mut matched = false;
        for (j, cand) in segments.iter().enumerate() {
            if added[j] {
                continue;
            }
            if cand.start != predicted.start
                || cand.end != predicted.end
                || cand.score != predicted.score
            {
                continue;
            }
            if cand.text.as_deref() != Some(pred_text) {
                continue;
            }
            let cand_kana = match compound_kana(&cand.word) {
                Ok(k) => k,
                Err(_) => continue,
            };
            if cand_kana != pred_kana {
                continue;
            }
            let cand_primary_seq = match compound_primary_seq(&cand.word) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if cand_primary_seq != pred_primary_seq {
                continue;
            }
            added[j] = true;
            matched = true;
            break;
        }
        if !matched {
            return Err(format!(
                "segments[{}] produces a segsplit (primary-seq={}, score={:?}, text={:?}) but no matching compound found in post-state",
                i, pred_primary_seq, predicted.score, pred_text
            ));
        }
    }
    Ok(added)
}

fn compound_kana(w: &KaniWordDispatchEnum) -> Result<&str, String> {
    match w {
        KaniWordDispatchEnum::Compound(c) => Ok(&c.kana),
        _ => Err("not a CompoundText".into()),
    }
}

fn compound_primary_seq(w: &KaniWordDispatchEnum) -> Result<i32, String> {
    match w {
        KaniWordDispatchEnum::Compound(c) => match c.primary.as_ref() {
            KaniWordDispatchEnum::Kanji(k) => Ok(k.seq),
            KaniWordDispatchEnum::Kana(k) => Ok(k.seq),
            KaniWordDispatchEnum::Proxy(p) => match p.source.as_ref() {
                kaniran_core::dict::kani::KaniSimpleTextDispatchEnum::Kanji(k) => Ok(k.seq),
                kaniran_core::dict::kani::KaniSimpleTextDispatchEnum::Kana(k) => Ok(k.seq),
                kaniran_core::dict::kani::KaniSimpleTextDispatchEnum::Proxy(_) => {
                    Err("nested proxy primary".into())
                }
            },
            _ => Err("compound primary not simple-text".into()),
        },
        _ => Err("not a CompoundText".into()),
    }
}

// ----- captured-SegmentList / Segment parsers (full: includes info) -------

fn parse_segment_list_full(value: &Value) -> Result<SegmentList, String> {
    let class = captured_class(value)?;
    if class != "SEGMENT-LIST" {
        return Err(format!("expected SEGMENT-LIST class, got :{}", class));
    }
    let segments: Vec<Segment> = match value.get("segments") {
        Some(Value::Array(arr)) => arr
            .iter()
            .map(parse_segment_full)
            .collect::<Result<_, _>>()
            .map_err(|err| format!("segments: {}", err))?,
        Some(Value::Null) | None => Vec::new(),
        Some(other) => return Err(format!("segments: expected array/null, got {}", other)),
    };
    let start = require_field(value, "start")?
        .as_i64()
        .ok_or_else(|| "start: missing/not int".to_string())? as usize;
    let end = require_field(value, "end")?
        .as_i64()
        .ok_or_else(|| "end: missing/not int".to_string())? as usize;
    let matches = require_field(value, "matches")?
        .as_i64()
        .ok_or_else(|| "matches: missing/not int".to_string())? as usize;
    Ok(SegmentList {
        segments,
        start,
        end,
        top: None,
        matches,
    })
}

fn parse_segment_full(value: &Value) -> Result<Segment, String> {
    let class = captured_class(value)?;
    if class != "SEGMENT" {
        return Err(format!("expected SEGMENT class, got :{}", class));
    }
    let start = require_field(value, "start")?
        .as_i64()
        .ok_or_else(|| format!("start not int: {}", value))? as usize;
    let end = require_field(value, "end")?
        .as_i64()
        .ok_or_else(|| format!("end not int: {}", value))? as usize;
    let word = parse_captured_word(require_field(value, "word")?)?;
    let score = match require_field(value, "score")? {
        Value::Null => None,
        Value::Number(n) => Some(
            n.as_i64()
                .ok_or_else(|| format!("score not i64: {}", n))? as i32,
        ),
        other => return Err(format!("score: expected number / null, got {}", other)),
    };
    let text = match require_field(value, "text")? {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        other => return Err(format!("text: expected string / null, got {}", other)),
    };
    let info = match require_field(value, "info")? {
        Value::Null => None,
        other => Some(parse_info_plist(other)?),
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

// ----- result comparison ---------------------------------------------------

fn compare_path_score_pair(
    actual: &(Vec<PathElement>, i32),
    captured_cons: &Value,
) -> Result<(), String> {
    let cons = captured_cons
        .get("_meta")
        .and_then(|m| m.get("cons"))
        .and_then(|c| c.as_array())
        .ok_or_else(|| format!("expected cons cell, got {}", captured_cons))?;
    if cons.len() != 2 {
        return Err(format!(
            "cons must have 2 elements, got {}: {}",
            cons.len(),
            captured_cons
        ));
    }
    let captured_path = &cons[0];
    let captured_score = cons[1]
        .as_i64()
        .ok_or_else(|| format!("cons tail (score): not int: {}", cons[1]))?
        as i32;
    if actual.1 != captured_score {
        return Err(format!(
            "score: rust={} lisp={}",
            actual.1, captured_score
        ));
    }
    let captured_elems: &[Value] = match captured_path {
        Value::Null => &[],
        Value::Array(arr) => arr.as_slice(),
        other => return Err(format!("cons head (path): expected array/null, got {}", other)),
    };
    if actual.0.len() != captured_elems.len() {
        return Err(format!(
            "path len: rust={} lisp={}",
            actual.0.len(),
            captured_elems.len()
        ));
    }
    for (i, (a, e)) in actual.0.iter().zip(captured_elems.iter()).enumerate() {
        compare_path_element(a, e).map_err(|err| format!("path[{}]: {}", i, err))?;
    }
    Ok(())
}

fn compare_path_element(actual: &PathElement, captured: &Value) -> Result<(), String> {
    let class = captured_class(captured)?;
    match (actual, class) {
        (PathElement::SegmentList(a), "SEGMENT-LIST") => compare_segment_list_all(a, captured),
        (PathElement::Synergy(a), "SYNERGY") => compare_synergy(a, captured),
        (PathElement::SegmentList(_), other) => {
            Err(format!("variant: rust=SEGMENT-LIST lisp=:{}", other))
        }
        (PathElement::Synergy(_), other) => Err(format!("variant: rust=SYNERGY lisp=:{}", other)),
    }
}

fn compare_synergy(actual: &Synergy, captured: &Value) -> Result<(), String> {
    let cap_desc = match require_field(captured, "description")? {
        Value::Null => None,
        Value::String(s) => Some(s.as_str()),
        other => return Err(format!("description: expected string/null, got {}", other)),
    };
    if actual.description.as_deref() != cap_desc {
        return Err(format!(
            "description: rust={:?} lisp={:?}",
            actual.description, cap_desc
        ));
    }
    let cap_conn = match require_field(captured, "connector")? {
        Value::Null => None,
        Value::String(s) => Some(s.as_str()),
        other => return Err(format!("connector: expected string/null, got {}", other)),
    };
    if actual.connector.as_deref() != cap_conn {
        return Err(format!(
            "connector: rust={:?} lisp={:?}",
            actual.connector, cap_conn
        ));
    }
    let cap_score = require_field(captured, "score")?
        .as_i64()
        .ok_or_else(|| "score not int".to_string())? as i32;
    if actual.score != cap_score {
        return Err(format!("score: rust={} lisp={}", actual.score, cap_score));
    }
    let cap_start = require_field(captured, "start")?
        .as_i64()
        .ok_or_else(|| "start not int".to_string())? as usize;
    if actual.start != cap_start {
        return Err(format!("start: rust={} lisp={}", actual.start, cap_start));
    }
    let cap_end = require_field(captured, "end")?
        .as_i64()
        .ok_or_else(|| "end not int".to_string())? as usize;
    if actual.end != cap_end {
        return Err(format!("end: rust={} lisp={}", actual.end, cap_end));
    }
    Ok(())
}

fn compare_segment_list_all(actual: &SegmentList, captured: &Value) -> Result<(), String> {
    let class = captured_class(captured)?;
    if class != "SEGMENT-LIST" {
        return Err(format!("expected SEGMENT-LIST class, got :{}", class));
    }
    let cap_start = require_field(captured, "start")?
        .as_i64()
        .ok_or_else(|| "start not int".to_string())? as usize;
    let cap_end = require_field(captured, "end")?
        .as_i64()
        .ok_or_else(|| "end not int".to_string())? as usize;
    let cap_matches = require_field(captured, "matches")?
        .as_i64()
        .ok_or_else(|| "matches not int".to_string())? as usize;
    if actual.start != cap_start {
        return Err(format!("start: rust={} lisp={}", actual.start, cap_start));
    }
    if actual.end != cap_end {
        return Err(format!("end: rust={} lisp={}", actual.end, cap_end));
    }
    if actual.matches != cap_matches {
        return Err(format!(
            "matches: rust={} lisp={}",
            actual.matches, cap_matches
        ));
    }
    let cap_segments = match require_field(captured, "segments")? {
        Value::Array(arr) => arr.as_slice(),
        Value::Null => &[][..],
        other => return Err(format!("segments: expected array/null, got {}", other)),
    };
    if actual.segments.len() != cap_segments.len() {
        return Err(format!(
            "segments len: rust={} lisp={}",
            actual.segments.len(),
            cap_segments.len()
        ));
    }
    for (i, (a, e)) in actual.segments.iter().zip(cap_segments.iter()).enumerate() {
        compare_segment_all(a, e).map_err(|err| format!("segments[{}]: {}", i, err))?;
    }
    Ok(())
}

fn compare_segment_all(actual: &Segment, captured: &Value) -> Result<(), String> {
    let class = captured_class(captured)?;
    if class != "SEGMENT" {
        return Err(format!("expected SEGMENT class, got :{}", class));
    }
    let cap_start = require_field(captured, "start")?
        .as_i64()
        .ok_or_else(|| "start not int".to_string())? as usize;
    let cap_end = require_field(captured, "end")?
        .as_i64()
        .ok_or_else(|| "end not int".to_string())? as usize;
    if actual.start != cap_start {
        return Err(format!("start: rust={} lisp={}", actual.start, cap_start));
    }
    if actual.end != cap_end {
        return Err(format!("end: rust={} lisp={}", actual.end, cap_end));
    }
    let cap_score = match require_field(captured, "score")? {
        Value::Null => None,
        Value::Number(n) => Some(
            n.as_i64()
                .ok_or_else(|| format!("score not i64: {}", n))? as i32,
        ),
        other => return Err(format!("score: expected number/null, got {}", other)),
    };
    if actual.score != cap_score {
        return Err(format!(
            "score: rust={:?} lisp={:?}",
            actual.score, cap_score
        ));
    }
    let cap_text = match require_field(captured, "text")? {
        Value::Null => None,
        Value::String(s) => Some(s.as_str()),
        other => return Err(format!("text: expected string/null, got {}", other)),
    };
    if actual.text.as_deref() != cap_text {
        return Err(format!(
            "text: rust={:?} lisp={:?}",
            actual.text, cap_text
        ));
    }
    let expected_word = parse_captured_word(require_field(captured, "word")?)?;
    compare_words(&actual.word, &expected_word)?;
    let cap_info_v = require_field(captured, "info")?;
    let cap_info: Option<KaniSegmentInfo> = match cap_info_v {
        Value::Null => None,
        other => Some(parse_info_plist(other)?),
    };
    match (&actual.info, &cap_info) {
        (None, None) => Ok(()),
        (Some(a), Some(e)) => compare_info(a, e),
        (a, e) => Err(format!(
            "info: rust={} lisp={}",
            if a.is_some() { "Some" } else { "None" },
            if e.is_some() { "Some" } else { "None" }
        )),
    }
}

fn compare_words(
    actual: &KaniWordDispatchEnum,
    expected: &KaniWordDispatchEnum,
) -> Result<(), String> {
    let a = format!("{:?}", actual);
    let e = format!("{:?}", expected);
    if a == e {
        Ok(())
    } else {
        Err(format!("word: rust={} lisp={}", a, e))
    }
}

// ----- info plist parsing + comparison (mirror get_seg_splits_test) -------

fn parse_info_plist(v: &Value) -> Result<KaniSegmentInfo, String> {
    let arr = v
        .as_array()
        .ok_or_else(|| format!("info: expected plist array, got {}", v))?;
    if arr.len() % 2 != 0 {
        return Err(format!("info: odd plist length {}", arr.len()));
    }
    let mut posi: Vec<String> = Vec::new();
    let mut seq_set: Vec<i32> = Vec::new();
    let mut conj: Vec<ConjData> = Vec::new();
    let mut common: Option<i32> = None;
    let mut score_info: KaniScoreInfo = KaniScoreInfo {
        prop_score: 0,
        kanji_break: Vec::new(),
        use_length_bonus: 0,
        split_info: KaniSplitInfo::None,
    };
    let mut kpcl: (bool, bool, bool, bool) = (false, false, false, false);
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

fn compare_info(actual: &KaniSegmentInfo, expected: &KaniSegmentInfo) -> Result<(), String> {
    let mut a = actual.posi.clone();
    let mut e = expected.posi.clone();
    a.sort();
    e.sort();
    if a != e {
        return Err(format!(
            "posi: rust={:?} lisp={:?}",
            actual.posi, expected.posi
        ));
    }
    if actual.seq_set != expected.seq_set {
        return Err(format!(
            "seq-set: rust={:?} lisp={:?}",
            actual.seq_set, expected.seq_set
        ));
    }
    if actual.conj.len() != expected.conj.len() {
        return Err(format!(
            "conj len: rust={} lisp={}",
            actual.conj.len(),
            expected.conj.len()
        ));
    }
    for (i, (a, e)) in actual.conj.iter().zip(expected.conj.iter()).enumerate() {
        compare_conj_data(a, e).map_err(|err| format!("conj[{}]: {}", i, err))?;
    }
    if actual.common != expected.common {
        return Err(format!(
            "common: rust={:?} lisp={:?}",
            actual.common, expected.common
        ));
    }
    compare_score_info(&actual.score_info, &expected.score_info)?;
    if actual.kpcl != expected.kpcl {
        return Err(format!(
            "kpcl: rust={:?} lisp={:?}",
            actual.kpcl, expected.kpcl
        ));
    }
    Ok(())
}

fn compare_conj_data(actual: &ConjData, expected: &ConjData) -> Result<(), String> {
    if actual.seq != expected.seq {
        return Err(format!("seq: rust={:?} lisp={:?}", actual.seq, expected.seq));
    }
    if actual.from != expected.from {
        return Err(format!("from: rust={:?} lisp={:?}", actual.from, expected.from));
    }
    if actual.via != expected.via {
        return Err(format!("via: rust={:?} lisp={:?}", actual.via, expected.via));
    }
    match (&actual.prop, &expected.prop) {
        (None, None) => (),
        (Some(a), Some(e)) => {
            if a.id != e.id
                || a.conj_id != e.conj_id
                || a.conj_type != e.conj_type
                || a.pos != e.pos
                || a.neg != e.neg
                || a.fml != e.fml
            {
                return Err(format!("prop: rust={:?} lisp={:?}", a, e));
            }
        }
        (a, e) => {
            return Err(format!(
                "prop: rust={} lisp={}",
                if a.is_some() { "Some" } else { "None" },
                if e.is_some() { "Some" } else { "None" }
            ))
        }
    }
    if actual.src_map != expected.src_map {
        return Err(format!(
            "src_map: rust={:?} lisp={:?}",
            actual.src_map, expected.src_map
        ));
    }
    Ok(())
}

fn compare_score_info(actual: &KaniScoreInfo, expected: &KaniScoreInfo) -> Result<(), String> {
    if actual.prop_score != expected.prop_score {
        return Err(format!(
            "score-info.prop_score: rust={} lisp={}",
            actual.prop_score, expected.prop_score
        ));
    }
    if actual.kanji_break != expected.kanji_break {
        return Err(format!(
            "score-info.kanji_break: rust={:?} lisp={:?}",
            actual.kanji_break, expected.kanji_break
        ));
    }
    if actual.use_length_bonus != expected.use_length_bonus {
        return Err(format!(
            "score-info.use_length_bonus: rust={} lisp={}",
            actual.use_length_bonus, expected.use_length_bonus
        ));
    }
    match (&actual.split_info, &expected.split_info) {
        (KaniSplitInfo::None, KaniSplitInfo::None) => Ok(()),
        (KaniSplitInfo::Score(a), KaniSplitInfo::Score(e)) if a == e => Ok(()),
        (
            KaniSplitInfo::Parts { score_mod: ams, part_scores: aps },
            KaniSplitInfo::Parts { score_mod: ems, part_scores: eps },
        ) if ams == ems && aps == eps => Ok(()),
        (a, e) => Err(format!("score-info.split_info: rust={:?} lisp={:?}", a, e)),
    }
}

fn require_field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field {} on: {}", key, value))
}

#[tokio::main]
async fn main() {
    // Temporary profiling modes:
    //   KANI_PROFILE=<svg-path>     → profile row 0 only
    //   KANI_PROFILE_ALL=<svg-path> → profile every row, sequentially
    // Both run under a pprof sampling guard and write a flamegraph SVG.
    let prof_one = std::env::var("KANI_PROFILE").ok();
    let prof_all = std::env::var("KANI_PROFILE_ALL").ok();
    if prof_one.is_some() || prof_all.is_some() {
        let parquet = common::parse_path_arg();
        let captured = common::load_parquet(&parquet);
        let ctx = common::setup_ctx().await;
        let guard = pprof::ProfilerGuardBuilder::default()
            .frequency(1000)
            .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build()
            .expect("pprof guard");
        let t0 = std::time::Instant::now();
        let mut pass = 0usize;
        let mut fail = 0usize;
        let mut first_failures: Vec<String> = Vec::new();
        let rows: &[_] = if prof_all.is_some() {
            captured.rows.as_slice()
        } else {
            &captured.rows[..1]
        };
        for (idx, row) in rows.iter().enumerate() {
            match audit_one(&ctx, row).await {
                Ok(()) => pass += 1,
                Err(err) => {
                    fail += 1;
                    if first_failures.len() < 10 {
                        first_failures.push(format!("[row {}] {}", idx + 1, err));
                    }
                }
            }
        }
        let elapsed = t0.elapsed();
        eprintln!(
            "profile: {} rows in {:.2}s pass={} fail={}",
            rows.len(),
            elapsed.as_secs_f64(),
            pass,
            fail,
        );
        for f in &first_failures {
            eprintln!("  {}", f);
        }
        let prof_path = prof_all.or(prof_one).unwrap();
        let report = guard.report().build().expect("pprof report");
        let file = std::fs::File::create(&prof_path).expect("create flamegraph file");
        report.flamegraph(file).expect("write flamegraph");
        eprintln!("wrote flamegraph: {}", prof_path);
        return;
    }
    common::run_async_streaming_with_post_hook(
        EXPECTED_FQN,
        audit_one,
        kaniran_core::dict::kani::assert_field_coverage,
    )
    .await;
}
