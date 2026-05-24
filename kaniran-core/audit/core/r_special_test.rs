//! Manual fixture-replay runner for `ICHIRAN:R-SPECIAL`.
//! Source under test: `src/core/r_special.rs`.
//!
//! Run with:
//!   cargo run --bin r_special_test -- \
//!       --path corpus/<corpus_tag>/r_special.parquet
//!
//! Args are `[<method>, "word"]`; the one result value is the special
//! romanization or `null` (no special case).

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::core::r_special::r_special;

use common::{single_result, with_captured_method, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN:R-SPECIAL";

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    let method_value = row.args.first().ok_or("expected 2 args")?;
    let word = row
        .args
        .get(1)
        .and_then(|value| value.as_str())
        .ok_or("arg 1 not a string")?;
    let result = single_result(&row.result)?;
    let expected: Option<String> = match result {
        serde_json::Value::Null => None,
        serde_json::Value::String(text) => Some(text.clone()),
        other => return Err(format!("result not a string / null: {other}")),
    };

    with_captured_method(method_value, |method| {
        let actual = r_special(method, word);
        if actual == expected {
            Ok(())
        } else {
            Err(format!("\n  rust: {actual:?}\n  lisp: {expected:?}"))
        }
    })
}

fn main() {
    common::run_sync(EXPECTED_FQN, audit_one);
}
