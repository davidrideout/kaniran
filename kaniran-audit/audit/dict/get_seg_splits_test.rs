//! Manual fixture-replay runner for `ICHIRAN/DICT:GET-SEG-SPLITS`.
//! Source under test: `src/dict/get_seg_splits.rs`.
//!
//! Run with:
//!   cargo run --release --bin get_seg_splits_test -- \
//!       --path corpus/extracted_chunk_b_segmentation_2026_05_14/dict/get_seg_splits.parquet
//!
//! Replays a left/right segment-list pair through `get_seg_splits` and
//! compares the returned outer splits (lists of segment-list/synergy
//! values) against the Lisp result.

#[path = "../common/mod.rs"]
mod common;

use std::sync::Arc;

use serde_json::Value;

use kaniran_core::dict::conj_data_struct::ConjData;
use kaniran_core::dict::get_seg_splits::get_seg_splits;
use kaniran_core::dict::kani_lite_segment_list::KaniLiteSegmentList;
use kaniran_core::dict::kani_lite_top_array_item::KaniLitePathElement;
use kaniran_core::dict::kani_word::KaniWordDispatchEnum;
use kaniran_core::dict::segment_list_struct::SegmentList;
use kaniran_core::dict::segment_struct::{
    KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment,
};
use kaniran_core::dict::synergy_struct::Synergy;
use kaniran_core::dict::top_array_item_struct::PathElement;

use common::{
    captured_class, parse_captured_word, parse_conj_list, parse_int_list, parse_kpcl,
    parse_opt_i32, parse_score_info, parse_string_list, single_result, CapturedRow,
};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:GET-SEG-SPLITS";

async fn audit_one(
    _ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.len() != 2 {
        return Err(format!(
            "expected 2 args (segment-list-left, segment-list-right), got {}",
            row.args.len()
        ));
    }
    let seg_left = parse_segment_list_full(&row.args[0])
        .map_err(|err| format!("arg 0 (seg-left): {}", err))?;
    let seg_right = parse_segment_list_full(&row.args[1])
        .map_err(|err| format!("arg 1 (seg-right): {}", err))?;

    let lite_left = Arc::new(KaniLiteSegmentList::from_segment_list(&seg_left));
    let lite_right = Arc::new(KaniLiteSegmentList::from_segment_list(&seg_right));
    let lite_result = get_seg_splits(&lite_left, &lite_right);
    // Materialize lite path elements back to full PathElement so the
    // existing captured-shape comparators still apply (parquet predates
    // the lite refactor).
    let actual: Vec<Vec<PathElement>> = lite_result
        .into_iter()
        .map(|path| {
            path.into_iter()
                .map(|elem| match elem {
                    KaniLitePathElement::SegmentList(lite) => {
                        PathElement::SegmentList(lite.to_segment_list())
                    }
                    KaniLitePathElement::Synergy(s) => PathElement::Synergy(s),
                })
                .collect()
        })
        .collect();

    let expected_value = single_result(&row.result)?;
    let expected_outer: &[Value] = match expected_value {
        Value::Null => &[],
        Value::Array(arr) => arr.as_slice(),
        other => return Err(format!("result[0]: expected array / null, got {}", other)),
    };
    if actual.len() != expected_outer.len() {
        return Err(format!(
            "outer-split count: rust={} lisp={}",
            actual.len(),
            expected_outer.len()
        ));
    }
    for (idx, (actual_path, expected_path_v)) in
        actual.iter().zip(expected_outer.iter()).enumerate()
    {
        let expected_path = expected_path_v.as_array().ok_or_else(|| {
            format!(
                "outer[{}]: expected array of path-elements, got {}",
                idx, expected_path_v
            )
        })?;
        if actual_path.len() != expected_path.len() {
            return Err(format!(
                "outer[{}]: element count rust={} lisp={}",
                idx,
                actual_path.len(),
                expected_path.len()
            ));
        }
        for (eidx, (actual_elem, expected_elem)) in
            actual_path.iter().zip(expected_path.iter()).enumerate()
        {
            compare_path_element(actual_elem, expected_elem)
                .map_err(|err| format!("outer[{}][{}]: {}", idx, eidx, err))?;
        }
    }
    Ok(())
}

// ----- segment-list parser (full: includes per-segment info) -----------------
//
// Mirror of `audit/dict/get_seg_initial_test.rs:parse_segment_full` —
// kept inline so the audit binary is self-contained (per gen_score
// L144's "preserve audit-binary independence" note).

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
        Some(other) => {
            return Err(format!("segments: expected array / null, got {}", other))
        }
    };
    let start = value
        .get("start")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "start: missing / not int".to_string())? as usize;
    let end = value
        .get("end")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "end: missing / not int".to_string())? as usize;
    let matches = value
        .get("matches")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "matches: missing / not int".to_string())? as usize;
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

// ----- path-element / segment / segment-list / synergy comparators ----------

fn compare_path_element(actual: &PathElement, captured: &Value) -> Result<(), String> {
    let class = captured_class(captured)?;
    match (actual, class) {
        (PathElement::SegmentList(a), "SEGMENT-LIST") => compare_segment_list_all(a, captured),
        (PathElement::Synergy(a), "SYNERGY") => compare_synergy(a, captured),
        (PathElement::SegmentList(_), other) => Err(format!(
            "variant: rust=SEGMENT-LIST lisp=:{}",
            other
        )),
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
        return Err(format!(
            "start: rust={} lisp={}",
            actual.start, cap_start
        ));
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
        return Err(format!(
            "start: rust={} lisp={}",
            actual.start, cap_start
        ));
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

// ----- info plist parsing + comparison -------------------------------------
//
// Mirror of `audit/dict/get_seg_initial_test.rs` (also kept inline per
// the gen_score precedent).

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
        return Err(format!(
            "seq: rust={:?} lisp={:?}",
            actual.seq, expected.seq
        ));
    }
    if actual.from != expected.from {
        return Err(format!(
            "from: rust={:?} lisp={:?}",
            actual.from, expected.from
        ));
    }
    if actual.via != expected.via {
        return Err(format!(
            "via: rust={:?} lisp={:?}",
            actual.via, expected.via
        ));
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
        (a, e) => Err(format!(
            "score-info.split_info: rust={:?} lisp={:?}",
            a, e
        )),
    }
}

fn require_field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field {} on: {}", key, value))
}

#[tokio::main]
async fn main() {
    // Stream rows batch-by-batch — chunk_b's get_seg_splits parquet is
    // ~15GB (the dedup parquet on .103 at 2026-05-19). Buffering the
    // whole file would OOM (`feedback_audit_must_stream`).
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
