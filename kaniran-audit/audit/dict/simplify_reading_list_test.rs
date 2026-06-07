//! Manual fixture-replay runner for `ICHIRAN/DICT:SIMPLIFY-READING-LIST`.
//! Source under test: `src/dict/simplify_reading_list.rs`.
//!
//! Run with:
//!   cargo run --release --bin simplify_reading_list_test -- \
//!       --path corpus/<corpus_tag>/dict/simplify_reading_list.parquet
//!
//! Replays a captured reading list through `simplify_reading_list` and
//! compares the returned simplified list against the Lisp result.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use common::{single_result, CapturedRow};
use kaniran_core::dict::conj::simplify_reading_list;

const EXPECTED_FQN: &str = "ICHIRAN/DICT:SIMPLIFY-READING-LIST";

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!(
            "expected 1 arg (reading-list), got {}",
            row.args.len()
        ));
    }
    let reading_list = parse_string_list(&row.args[0], "arg 0 (reading-list)")?;

    let actual = simplify_reading_list(&reading_list);

    let result = single_result(&row.result)?;
    let expected = parse_string_list(result, "result[0]")?;

    if actual == expected {
        Ok(())
    } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, expected))
    }
}

fn parse_string_list(value: &Value, label: &str) -> Result<Vec<String>, String> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => arr
            .iter()
            .map(|item| {
                item.as_str()
                    .map(|item_str| item_str.to_string())
                    .ok_or_else(|| format!("{}: element not string: {}", label, item))
            })
            .collect(),
        other => Err(format!("{}: expected array / null, got {}", label, other)),
    }
}

fn main() {
    common::run_sync(EXPECTED_FQN, audit_one);
}
