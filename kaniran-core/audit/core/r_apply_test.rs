//! Manual fixture-replay runner for `ICHIRAN:R-APPLY`.
//! Source under test: `src/core/r_apply.rs`.
//!
//! Run with:
//!   cargo run --bin r_apply_test -- \
//!       --path corpus/<corpus_tag>/r_apply.parquet
//!
//! Args are `[":MODIFIER", <method>, <cc-tree>]`; the one result value is
//! the romanized substring.

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::core::r_apply::r_apply;

use common::{parse_cc_tree, parse_kana_class, single_result, with_captured_method, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN:R-APPLY";

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    let modifier_keyword = row
        .args
        .first()
        .and_then(|value| value.as_str())
        .ok_or("arg 0 not a keyword")?;
    let modifier = parse_kana_class(modifier_keyword)?;
    let method_value = row.args.get(1).ok_or("expected 3 args")?;
    let cc_tree = parse_cc_tree(row.args.get(2).ok_or("expected 3 args")?)?;
    let expected = single_result(&row.result)?
        .as_str()
        .ok_or("result not a string")?
        .to_string();

    with_captured_method(method_value, |method| {
        let actual = r_apply(modifier, method, &cc_tree);
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
