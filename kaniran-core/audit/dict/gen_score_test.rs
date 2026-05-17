//! Manual fixture-replay runner for `ICHIRAN/DICT:GEN-SCORE`.
//! Source under test: `src/dict/gen_score.rs`.
//!
//! Run with:
//!   cargo run --release --bin gen_score_test -- \
//!       --path corpus/extracted_chunk_b_segmentation_2026_05_14/dict/gen_score.parquet
//!
//! Captured args shape (per `chunk_b_segmentation` corpus):
//!   `[<segment>, ":FINAL", <bool|null>, ":KANJI-BREAK", <int-list|null>]`
//!
//! Captured result shape:
//!   `[<mutated-segment>]` — the same segment with `score` and `info`
//!   populated by the recursive `calc-score` call. Every other slot
//!   (start, end, word, text) is preserved by gen-score's setf and
//!   passes through to the result; the audit compares the two
//!   mutated slots only.
//!
//! Streaming async runner because the parquet is 2.25M rows — the
//! default `run_async` would OOM during its in-memory group-by-args
//! dedup pass.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::conj_data_struct::ConjData;
use kaniran_core::dict::gen_score::gen_score;
use kaniran_core::dict::segment_struct::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo};

use common::{
    captured_class, parse_captured_segment, parse_conj_list, parse_int_list, parse_kpcl,
    parse_opt_i32, parse_score_info, parse_string_list, CapturedRow,
};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:GEN-SCORE";

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.is_empty() {
        return Err("gen-score args empty".into());
    }
    // The tracer projects args POST-call (see `feedback_extract_post_call_mutation_risk`),
    // so the captured segment already has the `score` / `info` slots
    // mutated by `gen-score` itself. That doesn't bias the audit: the
    // ported `gen_score` unconditionally overwrites both slots before
    // returning, so the captured values flow through as-is and the
    // comparison below checks the function's own re-computation
    // against the recorded result segment (the same mutated instance).
    let mut segment = parse_captured_segment(&row.args[0])
        .map_err(|e| format!("arg 0 (segment): {}", e))?;
    let (final_, kanji_break) = walk_keywords(&row.args[1..])?;

    gen_score(ctx, &mut segment, final_, &kanji_break)
        .await
        .map_err(|e| format!("gen_score: {}", e))?;

    if row.result.len() != 1 {
        return Err(format!("expected 1 result, got {}", row.result.len()));
    }
    let expected_segment = &row.result[0];
    let class = captured_class(expected_segment)?;
    if class != "SEGMENT" {
        return Err(format!("result not SEGMENT, got :{}", class));
    }

    // dict.lisp:986-987 — gen-score mutates `score` and `info` only.
    let expected_score = match expected_segment.get("score").unwrap_or(&Value::Null) {
        Value::Null => None,
        Value::Number(n) => Some(
            n.as_i64()
                .ok_or_else(|| format!("expected score not i64: {}", n))?
                as i32,
        ),
        other => return Err(format!("expected score: {}", other)),
    };
    if segment.score != expected_score {
        return Err(format!(
            "score: rust={:?} lisp={:?}",
            segment.score, expected_score
        ));
    }

    let expected_info_val = expected_segment.get("info").unwrap_or(&Value::Null);
    let expected_info_opt = match expected_info_val {
        Value::Null => None,
        other => Some(parse_info_plist(other)?),
    };
    match (&segment.info, &expected_info_opt) {
        (None, None) => Ok(()),
        (Some(a), Some(e)) => compare_info(a, e),
        (a, e) => Err(format!(
            "info: rust={} lisp={}",
            if a.is_some() { "Some" } else { "None" },
            if e.is_some() { "Some" } else { "None" },
        )),
    }
}

fn walk_keywords(tail: &[Value]) -> Result<(bool, Vec<usize>), String> {
    let mut i = 0;
    let mut final_ = false;
    let mut kanji_break: Vec<usize> = Vec::new();
    while i < tail.len() {
        let key = tail[i]
            .as_str()
            .ok_or_else(|| format!("keyword at {} not string: {}", i, tail[i]))?;
        if i + 1 >= tail.len() {
            return Err(format!("keyword {} missing value", key));
        }
        let v = &tail[i + 1];
        match key {
            ":FINAL" => {
                final_ = match v {
                    Value::Null => false,
                    Value::Bool(b) => *b,
                    other => return Err(format!(":FINAL: expected bool/null, got {}", other)),
                };
            }
            ":KANJI-BREAK" => {
                kanji_break = match v {
                    Value::Null => Vec::new(),
                    Value::Array(arr) => arr
                        .iter()
                        .map(|n| {
                            n.as_u64()
                                .map(|n| n as usize)
                                .ok_or_else(|| format!(":KANJI-BREAK entry: {}", n))
                        })
                        .collect::<Result<_, _>>()?,
                    other => {
                        return Err(format!(":KANJI-BREAK: expected array/null, got {}", other))
                    }
                };
            }
            other => return Err(format!("gen-score: unexpected keyword {}", other)),
        }
        i += 2;
    }
    Ok((final_, kanji_break))
}

// ----- info plist parsing + comparison (shared shape with calc-score
// audit; kept inline to preserve audit-binary independence) -----

/// Captured plists from the compound + skip-word branch carry only
/// `:CONJ` — upstream's `(setf (getf nil :conj) X)` at
/// `dict.lisp:785-786` synthesizes a one-key plist when the inner
/// `calc-score` returned just `0`. The Rust src side mirrors this by
/// defaulting the other five fields to zero/empty, so missing keys
/// here resolve to the same zero/empty values rather than erroring.
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
    Ok(KaniSegmentInfo { posi, seq_set, conj, common, score_info, kpcl })
}

fn compare_info(actual: &KaniSegmentInfo, expected: &KaniSegmentInfo) -> Result<(), String> {
    // posi: order-insensitive (get-non-arch-posi DB query is unordered).
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
        (a, e) => Err(format!(
            "score-info.split_info: rust={:?} lisp={:?}",
            a, e
        )),
    }
}


#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
