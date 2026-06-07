//! Manual fixture-replay runner for `ICHIRAN:R-SIMPLIFY`.
//! Source under test: `src/core/r_simplify.rs`.
//!
//! Run with:
//!   cargo run --bin r_simplify_test -- \
//!       --path corpus/<corpus_tag>/r_simplify.parquet
//!
//! Args are `[<method>, "str"]`; the first result value is the
//! simplified romanization string.

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::core::rules::r_simplify;

use common::{with_captured_method, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN:R-SIMPLIFY";

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    let method_value = row.args.first().ok_or("expected 2 args")?;
    let input = row
        .args
        .get(1)
        .and_then(|value| value.as_str())
        .ok_or("arg 1 not a string")?;
    let expected = row
        .result
        .first()
        .and_then(|value| value.as_str())
        .ok_or("result 0 not a string")?
        .to_string();

    with_captured_method(method_value, |method| {
        let actual = r_simplify(method, input);
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
