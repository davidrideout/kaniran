//! Manual fixture-replay runner for `ICHIRAN/DICT:COMPARE-COMMON`.
//! Source under test: `src/dict/compare_common.rs`.
//!
//! Run with:
//!   cargo run --bin compare_common_test -- \
//!       --path corpus/<corpus_tag>/dict/compare_common.parquet
//!
//! Sort predicate for two `common` ranking values: prefers the more
//! common (lower positive) of `c1`/`c2`, treating nil and zero
//! specially. Args are `[c1, c2]` (each null or an integer); the result
//! is `null`, `true`, or `c1`.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::compare_common::{compare_common, CompareCommonResult};

use common::{single_result, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:COMPARE-COMMON";

fn parse_common(v: &Value, field: &str) -> Result<Option<i64>, String> {
    match v {
        Value::Null => Ok(None),
        Value::Number(n) => {
            let i = n
                .as_i64()
                .ok_or_else(|| format!("{} int not i64: {}", field, n))?;
            Ok(Some(i))
        }
        other => Err(format!(
            "{}: expected null or int, got {}",
            field, other
        )),
    }
}

fn parse_result(v: &Value) -> Result<CompareCommonResult, String> {
    match v {
        Value::Null => Ok(CompareCommonResult::Nil),
        Value::Bool(true) => Ok(CompareCommonResult::True),
        // Lisp NIL would have serialized as `null`; defensive — never
        // observed in the captured corpus.
        Value::Bool(false) => Ok(CompareCommonResult::Nil),
        Value::Number(n) => {
            let i = n
                .as_i64()
                .ok_or_else(|| format!("c1 result int not i64: {}", n))?;
            Ok(CompareCommonResult::C1(i))
        }
        other => Err(format!(
            "result: expected null / true / int, got {}",
            other
        )),
    }
}

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 2 {
        return Err(format!("expected 2 args, got {}", row.args.len()));
    }
    let c1 = parse_common(&row.args[0], "c1")?;
    let c2 = parse_common(&row.args[1], "c2")?;

    let actual = compare_common(c1, c2);
    let expected = parse_result(single_result(&row.result)?)?;

    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "\n  args: c1={:?} c2={:?}\n  rust: {:?}\n  lisp: {:?}",
            c1, c2, actual, expected
        ))
    }
}

fn main() {
    common::run_sync(EXPECTED_FQN, audit_one);
}
