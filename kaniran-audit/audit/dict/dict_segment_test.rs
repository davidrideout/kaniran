//! Manual fixture-replay runner for `ICHIRAN/DICT:DICT-SEGMENT`.
//! Source under test: `src/dict/dict_segment.rs`.
//!
//! Run with:
//!   cargo run --release --bin dict_segment_test -- \
//!       --path corpus/<corpus_tag>/dict/dict_segment.parquet
//!
//! Args: `[<str>, ":LIMIT", <limit>]`.
//! Result: `[[<cons>...]]` — a list of `(word-info-list . score)` cons
//! cells, one per top path, descending by score.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use common::{compare_captured_word_info, single_result, CapturedRow};
use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::word_info::dict_segment;
use kaniran_core::dict::word_info::WordInfo;

const EXPECTED_FQN: &str = "ICHIRAN/DICT:DICT-SEGMENT";

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    // Args: [str, ":LIMIT", limit].
    if row.args.len() != 3 {
        return Err(format!(
            "expected 3 args (str, :LIMIT, limit), got {}",
            row.args.len()
        ));
    }
    let str = row.args[0]
        .as_str()
        .ok_or_else(|| format!("arg 0 (str): not string: {}", row.args[0]))?;
    let key = row.args[1]
        .as_str()
        .ok_or_else(|| format!("arg 1: expected \":LIMIT\" string, got {}", row.args[1]))?;
    if key != ":LIMIT" {
        return Err(format!("arg 1: expected :LIMIT keyword, got {}", key));
    }
    let limit = row.args[2]
        .as_i64()
        .ok_or_else(|| format!("arg 2 (limit): not int: {}", row.args[2]))? as usize;

    let actual = dict_segment(ctx, str, Some(limit))
        .await
        .map_err(|err| format!("dict_segment: {}", err))?;

    // Result: [[<cons>...]].
    let result_value = single_result(&row.result)?;
    let captured_paths: &[Value] = match result_value {
        Value::Null => &[],
        Value::Array(arr) => arr.as_slice(),
        other => return Err(format!("result[0]: expected array / null, got {}", other)),
    };
    if actual.len() != captured_paths.len() {
        return Err(format!(
            "path count: rust={} lisp={}",
            actual.len(),
            captured_paths.len()
        ));
    }
    for (idx, (actual_pair, captured_cons)) in actual.iter().zip(captured_paths).enumerate() {
        compare_path_score_pair(actual_pair, captured_cons)
            .map_err(|err| format!("path[{}]: {}", idx, err))?;
    }
    Ok(())
}

fn compare_path_score_pair(
    actual: &(Vec<WordInfo>, i32),
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
    let captured_word_info_list = &cons[0];
    let captured_score = cons[1]
        .as_i64()
        .ok_or_else(|| format!("cons tail (score): not int: {}", cons[1]))? as i32;
    if actual.1 != captured_score {
        return Err(format!("score: rust={} lisp={}", actual.1, captured_score));
    }
    let captured_word_infos: &[Value] = match captured_word_info_list {
        Value::Null => &[],
        Value::Array(arr) => arr.as_slice(),
        other => return Err(format!("cons head (word-info-list): expected array/null, got {}", other)),
    };
    if actual.0.len() != captured_word_infos.len() {
        return Err(format!(
            "word-info count: rust={} lisp={}",
            actual.0.len(),
            captured_word_infos.len()
        ));
    }
    for (idx, (actual_wi, captured_wi)) in actual.0.iter().zip(captured_word_infos).enumerate() {
        compare_captured_word_info(actual_wi, captured_wi)
            .map_err(|err| format!("word_info[{}]: {}", idx, err))?;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
