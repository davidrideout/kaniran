//! Manual fixture-replay runner for `ICHIRAN:ROMANIZE-LIST`.
//! Source under test: `src/core/romanize_list.rs`.
//!
//! Run with:
//!   cargo run --bin romanize_list_test -- \
//!       --path corpus/<corpus_tag>/romanize_list.parquet
//!
//! Args are `[<cc-list>, ":METHOD", <method>]`; the one result value is
//! the romanized string.

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::core::romanize_list::romanize_list;

use common::{parse_cc_list, single_result, with_captured_method, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN:ROMANIZE-LIST";

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    let cc_list = parse_cc_list(row.args.first().ok_or("expected 3 args")?)?;
    let method_value = row.args.get(2).ok_or("expected 3 args")?;
    let expected = single_result(&row.result)?
        .as_str()
        .ok_or("result not a string")?
        .to_string();

    with_captured_method(method_value, |method| {
        let actual = romanize_list(&cc_list, method);
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
