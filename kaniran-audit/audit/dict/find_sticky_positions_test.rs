//! Manual fixture-replay runner for `ICHIRAN/DICT:FIND-STICKY-POSITIONS`.
//! Source under test: `src/dict/find_sticky_positions.rs`.
//!
//! Run with:
//!   cargo run --bin find_sticky_positions_test -- \
//!       --path corpus/<corpus_tag>/dict/find_sticky_positions.parquet
//!
//! Lisp signature: `(find-sticky-positions str)`. Result is the
//! one-element list `[(int int ...)]` — character positions.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::find_sticky_positions::find_sticky_positions;

use common::{single_result, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:FIND-STICKY-POSITIONS";

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!("expected 1 arg, got {}", row.args.len()));
    }
    let input = match &row.args[0] {
        Value::String(s) => s.as_str(),
        other => return Err(format!("arg 0: expected string, got {}", other)),
    };

    let actual = find_sticky_positions(input);

    let result = single_result(&row.result)?;
    let expected = parse_expected(result)?;

    if actual == expected {
        Ok(())
    } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, expected))
    }
}

fn parse_expected(v: &Value) -> Result<Vec<usize>, String> {
    if v.is_null() {
        return Ok(Vec::new());
    }
    let arr = v
        .as_array()
        .ok_or_else(|| format!("result not list: {}", v))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let n = item
            .as_u64()
            .ok_or_else(|| format!("position not non-negative integer: {}", item))?;
        out.push(n as usize);
    }
    Ok(out)
}

fn main() {
    common::run_sync(EXPECTED_FQN, audit_one);
}
