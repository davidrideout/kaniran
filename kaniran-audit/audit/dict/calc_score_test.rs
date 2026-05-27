//! Manual fixture-replay runner for `ICHIRAN/DICT:CALC-SCORE`.
//! Source under test: `src/dict/calc_score.rs`.
//!
//! Run with:
//!   cargo run --release --bin calc_score_test -- \
//!       --path corpus/extracted_chunk_b_segmentation_2026_05_14/dict/calc_score.parquet
//!
//! Captured args shape (per `chunk_b_segmentation` corpus):
//!   `[<word>, ":FINAL", <bool|null>, ":KANJI-BREAK", <int-list|null>]`
//! plus optional `:USE-LENGTH <int|null>` and `:SCORE-MOD <int|int-list|null>`
//! when the encapsulated capture catches calc-score's own recursive
//! invocations (the compound branch at `dict.lisp:782-788` and the
//! split branch at `dict.lisp:966-970` both pass those keywords).
//!
//! Captured result shape:
//!   `[<score:int>, <info-plist>]` — `(values score info)`.
//! When the early-return-0 branch fires (`dict.lisp:858`), the result
//! is `[0]` (single value); info is absent — the audit accepts both
//! shapes.
//!
//! Streaming async runner because the chunk_b parquet is 545K rows; the
//! standard `run_async` buffers every row in RAM for the group-by-args
//! dedup pass, which would OOM here.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::calc_score::calc_score;
use kaniran_core::dict::compound_text_class::ScoreMod;
use kaniran_core::dict::segment_struct::{KaniSegmentInfo, KaniScoreInfo, KaniSplitInfo};

use common::{
    captured_class, parse_captured_word, parse_opt_bool, parse_opt_i32, short_plist, CapturedRow,
};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:CALC-SCORE";

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.is_empty() {
        return Err("calc-score args empty".into());
    }
    let word = parse_captured_word(&row.args[0])
        .map_err(|e| format!("arg 0 (word): {}", e))?;
    let (final_, kanji_break, use_length, score_mod) = walk_keywords(&row.args[1..])?;

    let (actual_score, actual_info) = calc_score(
        ctx,
        &word,
        final_,
        use_length,
        score_mod.as_ref(),
        &kanji_break,
    )
    .await
    .map_err(|e| format!("calc_score: {}", e))?;

    let (expected_score, expected_info) = parse_result(&row.result)?;
    if actual_score != expected_score {
        return Err(format!(
            "score: rust={} lisp={}",
            actual_score, expected_score
        ));
    }
    compare_info_opt(actual_info.as_ref(), expected_info)
}

/// Parse the keyword tail. Top-level captures contain only `:FINAL`
/// and `:KANJI-BREAK`. Encapsulated recursive captures additionally
/// carry `:USE-LENGTH` and `:SCORE-MOD` (compound branch at
/// `dict.lisp:782-788`, split branch at `dict.lisp:966-970`). All four
/// are forwarded to the Rust [`calc_score`] call.
fn walk_keywords(
    tail: &[Value],
) -> Result<(bool, Vec<usize>, Option<i32>, Option<ScoreMod>), String> {
    let mut i = 0;
    let mut final_ = false;
    let mut kanji_break: Vec<usize> = Vec::new();
    let mut use_length: Option<i32> = None;
    let mut score_mod: Option<ScoreMod> = None;
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
                                .ok_or_else(|| format!(":KANJI-BREAK entry not uint: {}", n))
                        })
                        .collect::<Result<_, _>>()?,
                    other => {
                        return Err(format!(":KANJI-BREAK: expected array/null, got {}", other))
                    }
                };
            }
            ":USE-LENGTH" => {
                use_length = match v {
                    Value::Null => None,
                    Value::Number(n) => Some(
                        n.as_i64()
                            .ok_or_else(|| format!(":USE-LENGTH not i64: {}", n))?
                            as i32,
                    ),
                    other => {
                        return Err(format!(":USE-LENGTH: expected int/null, got {}", other))
                    }
                };
            }
            ":SCORE-MOD" => {
                // Match the upstream three score-mod shapes per `apply-score-mod`
                // (`dict.lisp:735-742`): integer → Single; list of ints → Stack of
                // Single; null → upstream `nil` ≡ `0` (CONVENTIONS §4.4 short-circuit
                // — both apply-score-mod methods produce 0 for these shapes).
                score_mod = parse_captured_score_mod(v)?;
            }
            other => return Err(format!("calc-score: unexpected keyword {}", other)),
        }
        i += 2;
    }
    Ok((final_, kanji_break, use_length, score_mod))
}

fn parse_captured_score_mod(v: &Value) -> Result<Option<ScoreMod>, String> {
    match v {
        Value::Null => Ok(None),
        Value::Number(n) => Ok(Some(ScoreMod::Single(
            n.as_i64()
                .ok_or_else(|| format!(":SCORE-MOD int not i64: {}", n))?,
        ))),
        Value::Array(arr) => {
            let mut items = Vec::with_capacity(arr.len());
            for elt in arr {
                let i = elt.as_i64().ok_or_else(|| {
                    format!(
                        ":SCORE-MOD list entry not i64 (likely a (constantly N) closure \
                         — projector clause for function-typed slots missing): {}",
                        elt
                    )
                })?;
                items.push(ScoreMod::Single(i));
            }
            Ok(Some(ScoreMod::Stack(items)))
        }
        other => Err(format!(":SCORE-MOD: expected int/list/null, got {}", other)),
    }
}

/// Result shape: `[score]` for the early-return-0 branch, or
/// `[score, info-plist]` for the normal terminal `(values score info)`.
fn parse_result(result: &[Value]) -> Result<(i32, Option<&Value>), String> {
    match result.len() {
        1 => {
            let s = result[0]
                .as_i64()
                .ok_or_else(|| format!("result[0] not int: {}", result[0]))?
                as i32;
            Ok((s, None))
        }
        2 => {
            let s = result[0]
                .as_i64()
                .ok_or_else(|| format!("result[0] not int: {}", result[0]))?
                as i32;
            Ok((s, Some(&result[1])))
        }
        n => Err(format!("expected 1 or 2 result values, got {}", n)),
    }
}

fn compare_info_opt(
    actual: Option<&KaniSegmentInfo>,
    expected: Option<&Value>,
) -> Result<(), String> {
    match (actual, expected) {
        (None, None) => Ok(()),
        (Some(a), Some(e)) => compare_info(a, e),
        (None, Some(e)) => Err(format!(
            "info: rust=None lisp present ({})",
            short_plist(e)
        )),
        (Some(a), None) => Err(format!("info: rust present (posi={:?}) lisp=None", a.posi)),
    }
}

/// Plist comparator. The Lisp info is a flat list of `(:KEY VAL :KEY
/// VAL …)`; walk pairs and compare each section.
///
/// Captured plists from the compound + skip-word branch carry only
/// `:CONJ` — upstream's `(setf (getf nil :conj) X)` synthesizes a
/// one-key plist (see `src/dict/calc_score.rs` compound arm and
/// `dict.lisp:785-786`). The src side matches this by defaulting the
/// other five fields to zero/empty, so we only compare keys that are
/// actually present and trust the src-side defaults for the rest.
fn compare_info(actual: &KaniSegmentInfo, expected: &Value) -> Result<(), String> {
    let arr = expected
        .as_array()
        .ok_or_else(|| format!("info: expected plist array, got {}", expected))?;
    if arr.len() % 2 != 0 {
        return Err(format!("info: odd plist length {}", arr.len()));
    }
    let mut i = 0;
    while i < arr.len() {
        let key = arr[i]
            .as_str()
            .ok_or_else(|| format!("info plist key at {} not string: {}", i, arr[i]))?;
        let val = &arr[i + 1];
        match key {
            ":POSI" => compare_string_list(&actual.posi, val, "posi")?,
            ":SEQ-SET" => compare_int_list(&actual.seq_set, val, "seq-set")?,
            ":CONJ" => compare_conj(&actual.conj, val)?,
            ":COMMON" => compare_common(actual.common, val)?,
            ":SCORE-INFO" => compare_score_info(&actual.score_info, val)?,
            ":KPCL" => compare_kpcl(actual.kpcl, val)?,
            other => return Err(format!("unknown info key {}", other)),
        }
        i += 2;
    }
    Ok(())
}

fn compare_string_list(actual: &[String], val: &Value, field: &str) -> Result<(), String> {
    let expected: Vec<String> = match val {
        Value::Null => Vec::new(),
        Value::Array(arr) => arr
            .iter()
            .map(|v| {
                v.as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| format!("{}: entry not string: {}", field, v))
            })
            .collect::<Result<_, _>>()?,
        other => return Err(format!("{}: expected array/null, got {}", field, other)),
    };
    let mut a: Vec<&String> = actual.iter().collect();
    let mut e: Vec<&String> = expected.iter().collect();
    a.sort();
    e.sort();
    if a != e {
        return Err(format!("{}: rust={:?} lisp={:?}", field, actual, expected));
    }
    Ok(())
}

fn compare_int_list(actual: &[i32], val: &Value, field: &str) -> Result<(), String> {
    let expected: Vec<i32> = match val {
        Value::Null => Vec::new(),
        Value::Array(arr) => arr
            .iter()
            .map(|v| {
                v.as_i64()
                    .map(|n| n as i32)
                    .ok_or_else(|| format!("{}: entry not int: {}", field, v))
            })
            .collect::<Result<_, _>>()?,
        other => return Err(format!("{}: expected array/null, got {}", field, other)),
    };
    if actual != expected.as_slice() {
        return Err(format!("{}: rust={:?} lisp={:?}", field, actual, expected));
    }
    Ok(())
}

fn compare_common(actual: Option<i32>, val: &Value) -> Result<(), String> {
    let expected = match val {
        Value::Null => None,
        Value::String(s) if s == ":NULL" => None,
        Value::Number(n) => Some(
            n.as_i64()
                .ok_or_else(|| format!("common not i64: {}", n))? as i32,
        ),
        other => return Err(format!("common: expected null/int, got {}", other)),
    };
    if actual != expected {
        return Err(format!("common: rust={:?} lisp={:?}", actual, expected));
    }
    Ok(())
}

fn compare_score_info(actual: &KaniScoreInfo, val: &Value) -> Result<(), String> {
    let arr = val
        .as_array()
        .ok_or_else(|| format!("score-info: expected array, got {}", val))?;
    if arr.len() != 4 {
        return Err(format!("score-info: expected 4-tuple, got {}", arr.len()));
    }
    // [prop-score, kanji-break, use-length-bonus, split-info]
    let exp_prop_score = arr[0]
        .as_i64()
        .ok_or_else(|| format!("score-info[0] not int: {}", arr[0]))? as i32;
    if actual.prop_score != exp_prop_score {
        return Err(format!(
            "score-info.prop_score: rust={} lisp={}",
            actual.prop_score, exp_prop_score
        ));
    }
    let exp_kanji_break: Vec<usize> = match &arr[1] {
        Value::Null => Vec::new(),
        Value::Array(arr) => arr
            .iter()
            .map(|v| {
                v.as_u64()
                    .map(|n| n as usize)
                    .ok_or_else(|| format!("score-info kanji-break entry not uint: {}", v))
            })
            .collect::<Result<_, _>>()?,
        other => {
            return Err(format!(
                "score-info[1]: expected array/null, got {}",
                other
            ))
        }
    };
    if actual.kanji_break != exp_kanji_break {
        return Err(format!(
            "score-info.kanji_break: rust={:?} lisp={:?}",
            actual.kanji_break, exp_kanji_break
        ));
    }
    let exp_ulb = arr[2]
        .as_i64()
        .ok_or_else(|| format!("score-info[2] not int: {}", arr[2]))? as i32;
    if actual.use_length_bonus != exp_ulb {
        return Err(format!(
            "score-info.use_length_bonus: rust={} lisp={}",
            actual.use_length_bonus, exp_ulb
        ));
    }
    compare_split_info(&actual.split_info, &arr[3])
}

fn compare_split_info(actual: &KaniSplitInfo, val: &Value) -> Result<(), String> {
    match (actual, val) {
        (KaniSplitInfo::None, Value::Null) => Ok(()),
        (KaniSplitInfo::Score(n), Value::Number(m)) => {
            let exp = m
                .as_i64()
                .ok_or_else(|| format!("split-info int not i64: {}", m))? as i32;
            if *n != exp {
                Err(format!("split-info Score: rust={} lisp={}", n, exp))
            } else {
                Ok(())
            }
        }
        (KaniSplitInfo::Parts { score_mod, part_scores }, Value::Array(arr)) => {
            // Lisp `(cons score-mod-split part-scores)` → JSON
            // `[score_mod, p0, p1, …]`.
            if arr.is_empty() {
                return Err("split-info Parts: lisp array empty".into());
            }
            let exp_score_mod = arr[0]
                .as_i64()
                .ok_or_else(|| format!("split-info[0] not int: {}", arr[0]))?
                as i32;
            if *score_mod != exp_score_mod {
                return Err(format!(
                    "split-info Parts.score_mod: rust={} lisp={}",
                    score_mod, exp_score_mod
                ));
            }
            let exp_parts: Vec<i32> = arr[1..]
                .iter()
                .map(|v| {
                    v.as_i64()
                        .map(|n| n as i32)
                        .ok_or_else(|| format!("split-info part not int: {}", v))
                })
                .collect::<Result<_, _>>()?;
            if part_scores != &exp_parts {
                return Err(format!(
                    "split-info Parts.part_scores: rust={:?} lisp={:?}",
                    part_scores, exp_parts
                ));
            }
            Ok(())
        }
        (a, e) => Err(format!(
            "split-info: rust={:?} lisp={}",
            a,
            short_plist(e)
        )),
    }
}

fn compare_kpcl(
    actual: (bool, bool, bool, bool),
    val: &Value,
) -> Result<(), String> {
    let arr = val
        .as_array()
        .ok_or_else(|| format!("kpcl: expected array, got {}", val))?;
    if arr.len() != 4 {
        return Err(format!("kpcl: expected 4-tuple, got {}", arr.len()));
    }
    // kpcl entries: upstream `(list (or kanji-p katakana-p) primary-p
    // common-p long-p)` stores the raw CL `or`/`and` result, which is
    // the first truthy operand — not necessarily T. E.g. primary-p can
    // be the list `("pn")` from `(and common-p pronoun-p)` (pronoun-p
    // = `(member "pn" posi)`), or the seq-set list from `cop-da-p =
    // (intersection seq-set *copulae*)`. All consumers read these
    // slots as truthy predicates; collapse any non-nil JSON value to
    // `true` (matches `require_bool` in common/mod.rs).
    let bools: Vec<bool> = arr
        .iter()
        .map(|v| match v {
            Value::Null => false,
            Value::Bool(b) => *b,
            _ => true,
        })
        .collect();
    let exp = (bools[0], bools[1], bools[2], bools[3]);
    if actual != exp {
        return Err(format!("kpcl: rust={:?} lisp={:?}", actual, exp));
    }
    Ok(())
}

fn compare_conj(
    actual: &[kaniran_core::dict::conj_data_struct::ConjData],
    val: &Value,
) -> Result<(), String> {
    let arr = match val {
        Value::Null => &[][..],
        Value::Array(arr) => arr.as_slice(),
        other => return Err(format!("conj: expected array/null, got {}", other)),
    };
    if actual.len() != arr.len() {
        return Err(format!(
            "conj: rust len={} lisp len={}",
            actual.len(),
            arr.len()
        ));
    }
    for (idx, (a, e)) in actual.iter().zip(arr.iter()).enumerate() {
        compare_conj_data(a, e).map_err(|err| format!("conj[{}]: {}", idx, err))?;
    }
    Ok(())
}

fn compare_conj_data(
    actual: &kaniran_core::dict::conj_data_struct::ConjData,
    val: &Value,
) -> Result<(), String> {
    let class = captured_class(val)?;
    if class != "CONJ-DATA" {
        return Err(format!("expected CONJ-DATA, got :{}", class));
    }
    let exp_seq = parse_opt_i32(val.get("seq").unwrap_or(&Value::Null), "seq")?;
    if actual.seq != exp_seq {
        return Err(format!("seq: rust={:?} lisp={:?}", actual.seq, exp_seq));
    }
    let exp_from = parse_opt_i32(val.get("from").unwrap_or(&Value::Null), "from")?;
    if actual.from != exp_from {
        return Err(format!("from: rust={:?} lisp={:?}", actual.from, exp_from));
    }
    let exp_via = parse_opt_i32(val.get("via").unwrap_or(&Value::Null), "via")?;
    if actual.via != exp_via {
        return Err(format!("via: rust={:?} lisp={:?}", actual.via, exp_via));
    }
    // prop
    let prop_val = val.get("prop").unwrap_or(&Value::Null);
    match (&actual.prop, prop_val) {
        (None, Value::Null) => (),
        (Some(a), v) if !v.is_null() => compare_conj_prop(a, v)?,
        (a, e) => {
            return Err(format!(
                "prop: rust={} lisp={}",
                if a.is_some() { "Some" } else { "None" },
                short_plist(e)
            ))
        }
    }
    // src_map
    let src_map_val = val.get("src_map").unwrap_or(&Value::Null);
    let exp_src_map: Vec<(String, String)> = match src_map_val {
        Value::Null => Vec::new(),
        Value::Array(arr) => arr
            .iter()
            .map(|pair| {
                let pair_arr = pair
                    .as_array()
                    .ok_or_else(|| format!("src_map entry not array: {}", pair))?;
                if pair_arr.len() != 2 {
                    return Err(format!("src_map pair not 2-elem: {}", pair));
                }
                let a = pair_arr[0]
                    .as_str()
                    .ok_or_else(|| format!("src_map[0] not string: {}", pair_arr[0]))?
                    .to_string();
                let b = pair_arr[1]
                    .as_str()
                    .ok_or_else(|| format!("src_map[1] not string: {}", pair_arr[1]))?
                    .to_string();
                Ok((a, b))
            })
            .collect::<Result<_, _>>()?,
        other => return Err(format!("src_map: expected array/null, got {}", other)),
    };
    if actual.src_map != exp_src_map {
        return Err(format!(
            "src_map: rust={:?} lisp={:?}",
            actual.src_map, exp_src_map
        ));
    }
    Ok(())
}

fn compare_conj_prop(
    actual: &kaniran_core::dict::conj_prop_dao::ConjProp,
    val: &Value,
) -> Result<(), String> {
    let class = captured_class(val)?;
    if class != "CONJ-PROP" {
        return Err(format!("expected CONJ-PROP, got :{}", class));
    }
    let exp_id = val.get("id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    if actual.id != exp_id {
        return Err(format!("conj-prop.id: rust={} lisp={}", actual.id, exp_id));
    }
    let exp_conj_id = val.get("conj_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    if actual.conj_id != exp_conj_id {
        return Err(format!(
            "conj-prop.conj_id: rust={} lisp={}",
            actual.conj_id, exp_conj_id
        ));
    }
    let exp_conj_type = val.get("conj_type").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    if actual.conj_type != exp_conj_type {
        return Err(format!(
            "conj-prop.conj_type: rust={} lisp={}",
            actual.conj_type, exp_conj_type
        ));
    }
    let exp_pos = val
        .get("pos")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if actual.pos != exp_pos {
        return Err(format!(
            "conj-prop.pos: rust={:?} lisp={:?}",
            actual.pos, exp_pos
        ));
    }
    let exp_neg = parse_opt_bool(val.get("neg").unwrap_or(&Value::Null), "neg")?;
    if actual.neg != exp_neg {
        return Err(format!(
            "conj-prop.neg: rust={:?} lisp={:?}",
            actual.neg, exp_neg
        ));
    }
    let exp_fml = parse_opt_bool(val.get("fml").unwrap_or(&Value::Null), "fml")?;
    if actual.fml != exp_fml {
        return Err(format!(
            "conj-prop.fml: rust={:?} lisp={:?}",
            actual.fml, exp_fml
        ));
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
