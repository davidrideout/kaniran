//! Manual fixture-replay runner for `ICHIRAN:R-BASE`.
//! Source under test: `src/core/r_base.rs`.
//!
//! Run with:
//!   cargo run --bin r_base_test -- \
//!       --path corpus/<corpus_tag>/r_base.parquet
//!
//! Args are `[<method>, ":KW"]` (the romanization method object and the
//! mora-class keyword); the one result value is the Latin spelling.

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::core::r_base::r_base;

use common::{parse_kana_class, single_result, with_captured_method, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN:R-BASE";

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    let method_value = row.args.first().ok_or("expected 2 args")?;
    let item_keyword = row
        .args
        .get(1)
        .and_then(|value| value.as_str())
        .ok_or("arg 1 not a keyword")?;
    let item = parse_kana_class(item_keyword)?;
    let expected = single_result(&row.result)?
        .as_str()
        .ok_or("result not a string")?
        .to_string();

    with_captured_method(method_value, |method| {
        let actual = r_base(method, item);
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
