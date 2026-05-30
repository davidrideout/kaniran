//! Manual fixture-replay runner for `ICHIRAN/DICT:FILL-SEGMENT-PATH`.
//! Source under test: `src/dict/fill_segment_path.rs`.
//!
//! Run with:
//!   cargo run --bin fill_segment_path_test -- \
//!       --path corpus/<corpus_tag>/dict/fill_segment_path.parquet
//!
//! Args: `(<str> [<path-element>...])` — where each path-element is a
//! SEGMENT-LIST or a SYNERGY. `args[1]` is the heterogeneous path
//! list the upstream walks.
//! Result: `([<word-info>...])` — single value: the list of word-infos
//! produced by the upstream.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use common::{
    compare_captured_word_info, parse_captured_path_element, single_result, CapturedRow,
};
use kaniran_core::dict::best_path::fill_segment_path;
use kaniran_core::dict::segment::PathElement;

const EXPECTED_FQN: &str = "ICHIRAN/DICT:FILL-SEGMENT-PATH";

async fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.len() != 2 {
        return Err(format!(
            "expected 2 args (str + path), got {}",
            row.args.len()
        ));
    }
    let str = row.args[0]
        .as_str()
        .ok_or_else(|| format!("arg 0 not string: {}", row.args[0]))?;
    let path_arr = match &row.args[1] {
        Value::Array(arr) => arr,
        Value::Null => return run_with_empty_path(ctx, str, &row.result).await,
        other => return Err(format!("arg 1: expected array / null, got {}", other)),
    };
    let mut path: Vec<PathElement> = path_arr
        .iter()
        .map(parse_captured_path_element)
        .collect::<Result<_, _>>()?;

    let actual = fill_segment_path(ctx, str, &mut path)
        .await
        .map_err(|err| format!("fill_segment_path: {}", err))?;

    compare_word_info_list(&actual, &row.result)
}

async fn run_with_empty_path(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    str: &str,
    result: &[Value],
) -> Result<(), String> {
    let actual = fill_segment_path(ctx, str, &mut [])
        .await
        .map_err(|err| format!("fill_segment_path: {}", err))?;
    compare_word_info_list(&actual, result)
}

fn compare_word_info_list(
    actual: &[kaniran_core::dict::word_info::WordInfo],
    result: &[Value],
) -> Result<(), String> {
    let expected = single_result(result)?;
    let expected_arr = match expected {
        Value::Null => &[][..],
        Value::Array(arr) => arr.as_slice(),
        other => return Err(format!("result[0]: expected array / null, got {}", other)),
    };
    if actual.len() != expected_arr.len() {
        return Err(format!(
            "word-info count: rust={} lisp={}",
            actual.len(),
            expected_arr.len()
        ));
    }
    for (idx, (a, e)) in actual.iter().zip(expected_arr).enumerate() {
        compare_captured_word_info(a, e)
            .map_err(|err| format!("word_info[{}]: {}", idx, err))?;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    // Stream rows batch-by-batch — fill_segment_path's bulk corpus
    // (~2.6M rows, 1.2GB) exceeds memory when buffered. Each captured
    // (str, path) is deterministic, so no dedup-by-args pass is needed.
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
